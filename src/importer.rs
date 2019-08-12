use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::error;
use std::fmt;
use std::fs;
use std::io::{self, Read};

use crc32fast;
use tobj;

use crate::viewport_renderer::{Index, Vertex};

#[derive(Debug, PartialEq)]
pub enum ImporterError {
    FileNotFound,
    PermissionDenied,
    InvalidStructure,
    Other,
}

impl fmt::Display for ImporterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImporterError::FileNotFound => write!(f, "File was not found."),
            ImporterError::InvalidStructure => write!(f, "The obj file is not valid."),
            ImporterError::PermissionDenied => write!(f, "Permission denied."),
            ImporterError::Other => write!(f, "Unexpected error happened."),
        }
    }
}

impl error::Error for ImporterError {}

impl From<io::Error> for ImporterError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => ImporterError::FileNotFound,
            io::ErrorKind::PermissionDenied => ImporterError::PermissionDenied,
            _ => ImporterError::Other,
        }
    }
}

impl From<tobj::LoadError> for ImporterError {
    fn from(_err: tobj::LoadError) -> Self {
        ImporterError::InvalidStructure
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
}

#[derive(Debug)]
pub struct FileMetadata {
    checksum: u32,
    last_modified: std::time::SystemTime,
}

/// `Importer` takes care of importing of obj files and caching of their
/// internal representations. It holds paths to files, their metadata
/// (checksums, timestamps) and models of parsed obj files.
#[derive(Debug, Default)]
pub struct Importer {
    path_metadata: HashMap<String, FileMetadata>,
    loaded_models: HashMap<u32, Vec<Model>>,
}

impl Importer {
    pub fn new() -> Self {
        Default::default()
    }

    /// Tries to import obj file from given `path`. If file was already imported
    /// and its timestamp is identical, parsed models are returned from cache.
    /// Otherwise, file is read, checksum calculated and cache is checked whether
    /// given file contents were already saved. If not, obj file is parsed and
    /// cached.
    pub fn import_obj(&mut self, path: &str) -> Result<Vec<Model>, ImporterError> {
        let mut file = fs::File::open(path)?;
        let file_modified = file
            .metadata()
            .and_then(|metadata| metadata.modified())
            .expect("obj file should return its modified timestamp");

        // If paths and timestamps match, we can just return cached models.
        if let Entry::Occupied(path_metadata) = self.path_metadata.entry(path.to_string()) {
            if path_metadata.get().last_modified == file_modified {
                return Ok(self
                    .loaded_models
                    .get(&path_metadata.get().checksum)
                    .expect("Should get loaded models by obj file's checksum")
                    .clone());
            }
        }

        let file_size = file.metadata().map(|m| m.len() as usize + 1).unwrap_or(0);
        let mut file_contents = Vec::with_capacity(file_size);
        file.read_to_end(&mut file_contents)?;
        let checksum = calculate_checksum(&file_contents);

        let models = match self.loaded_models.entry(checksum) {
            Entry::Occupied(loaded_model) => {
                self.path_metadata.insert(
                    path.to_string(),
                    FileMetadata {
                        checksum,
                        last_modified: file_modified,
                    },
                );

                loaded_model.get().clone()
            }
            Entry::Vacant(loaded_model) => {
                let (tobj_models, _) = obj_buf_into_tobj(&mut file_contents.as_slice())?;
                let models = tobj_to_internal(tobj_models);

                self.path_metadata.insert(
                    path.to_string(),
                    FileMetadata {
                        checksum,
                        last_modified: file_modified,
                    },
                );
                loaded_model.insert(models.clone());

                models
            }
        };

        Ok(models)
    }

    /// FIXME: This is a poor man's testing method for cache contents. It should
    /// be removed once cacher is removed from this structure and proper unit
    /// tests are written for it.
    pub fn is_cached(&self, path: &str, checksum: u32) -> bool {
        if self.path_metadata.contains_key(path) {
            self.path_metadata.get(path).expect("").checksum == checksum
                && self.loaded_models.contains_key(&checksum)
        } else {
            false
        }
    }
}

/// Converts contents of obj file into tobj representation. Materials are
/// ignored.
pub fn obj_buf_into_tobj(file_contents: &mut &[u8]) -> tobj::LoadResult {
    tobj::load_obj_buf(file_contents, |_| Ok((vec![], HashMap::new())))
}

/// Converts `tobj::Model` vector into vector of internal `Model` representations.
/// It expects valid `tobj::Model` representation, eg. number of positions
/// divisible by 3.
pub fn tobj_to_internal(tobj_models: Vec<tobj::Model>) -> Vec<Model> {
    let mut models = Vec::with_capacity(tobj_models.len());

    for model in tobj_models {
        let mut vertices = Vec::with_capacity(model.mesh.positions.len() / 3);

        for positions_chunk in model.mesh.positions.chunks_exact(3) {
            vertices.push(Vertex {
                position: positions_chunk
                    .try_into()
                    .expect("Should convert slice into array"),
            });
        }

        models.push(Model {
            name: model.name,
            vertices,
            indices: model.mesh.indices,
        });
    }

    models
}

pub fn calculate_checksum(string: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();

    hasher.update(string);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_tobj_model(indices: Vec<u32>, positions: Vec<f32>) -> tobj::Model {
        tobj::Model {
            name: String::from("Test model"),
            mesh: tobj::Mesh {
                indices,
                positions,
                material_id: None,
                normals: vec![],
                texcoords: vec![],
            },
        }
    }

    #[test]
    fn test_tobj_to_internal_returns_correct_representation_for_single_model() {
        let tobj_model = create_tobj_model(vec![1, 2], vec![6.0, 5.0, 4.0, 3.0, 2.0, 1.0]);
        let tobj_models = vec![tobj_model.clone()];
        let models = tobj_to_internal(tobj_models);

        assert_eq!(
            models,
            vec![Model {
                name: tobj_model.name,
                vertices: vec![
                    Vertex {
                        position: [6.0, 5.0, 4.0]
                    },
                    Vertex {
                        position: [3.0, 2.0, 1.0]
                    }
                ],
                indices: tobj_model.mesh.indices,
            }]
        );
    }

    #[test]
    fn test_tobj_to_internal_returns_correct_representation_for_multiple_models() {
        let tobj_model_1 = create_tobj_model(vec![1, 2], vec![6.0, 5.0, 4.0, 3.0, 2.0, 1.0]);
        let tobj_model_2 = create_tobj_model(vec![3, 4], vec![16.0, 15.0, 14.0, 13.0, 12.0, 11.0]);
        let tobj_models = vec![tobj_model_1.clone(), tobj_model_2.clone()];
        let models = tobj_to_internal(tobj_models);

        assert_eq!(
            models,
            vec![
                Model {
                    name: tobj_model_1.name,
                    vertices: vec![
                        Vertex {
                            position: [6.0, 5.0, 4.0]
                        },
                        Vertex {
                            position: [3.0, 2.0, 1.0]
                        }
                    ],
                    indices: tobj_model_1.mesh.indices,
                },
                Model {
                    name: tobj_model_2.name,
                    vertices: vec![
                        Vertex {
                            position: [16.0, 15.0, 14.0]
                        },
                        Vertex {
                            position: [13.0, 12.0, 11.0]
                        }
                    ],
                    indices: tobj_model_2.mesh.indices,
                }
            ]
        );
    }
}
