use std::collections::{HashMap, HashSet};

use nalgebra::base::Vector3;
use nalgebra::geometry::Point3;
use smallvec::{smallvec, SmallVec};

use crate::convert::{cast_u32, cast_usize};
use crate::geometry::{Face, Geometry, OrientedEdge, TriangleFace, UnorientedEdge};
use crate::mesh_topology_analysis;

/// Make sure all the faces are oriented the same way - have the same winding
/// (vertex order).
///
/// This function crawls the mesh geometry and flips all the faces, which are
/// not facing the same way as the previous faces in the process, starting with
/// the first face in the list. As a result, the entre mesh can end up facing
/// inwards (be entirely reverted). At the moment we have no tools to detect
/// such a case automatically, so we need to rely on the user to check it and
/// potentially revert winding of the entire mesh.
///
/// The algorithm relies on the fact that in a proper non-manifold mesh, each
/// oriented edge has exactly one (for watertight mesh geometry) or none (for
/// mesh patches) counter-edge oriented the opposite direction. It crawls the
/// geometry and if a face neighboring the current one doesn't have the proper
/// winding, it's being reverted and only then triggers checking its own
/// neighbors.
///
/// This method, doesn't flip the normals associated with the face vertices, as
/// there is no unambiguous way to do so automatically.

// FIXME: Flip also vertex normals if the visual/practical tests prove it's
// needed
pub fn synchronize_mesh_winding(
    geometry: &Geometry,
    unoriented_edges: &[UnorientedEdge],
    edge_to_face_topology: &HashMap<u32, SmallVec<[u32; 8]>>,
) -> Geometry {
    // All faces in the original mesh geometry
    let original_triangle_faces: Vec<_> = geometry
        .faces()
        .iter()
        .map(|face| match face {
            Face::Triangle(f) => *f,
        })
        .collect();

    // item index = face index; TRUE = the face was already checked, FALSE = the
    // face hasn't yet been checked
    let mut face_treatment_pattern = vec![false; original_triangle_faces.len()];

    // Faces to be checked for winding, determined by the orientation of the
    // OrientedEdge of the neighbor, which triggered the check. Current face has
    // to contain a reverted edge to have the proper winding, otherwise it has
    // to be reverted.
    let mut edge_face_stack: Vec<(OrientedEdge, u32)> =
        Vec::with_capacity(original_triangle_faces.len() / 2);

    // Faces already checked and reverted if needed => all the faces in this
    // list have the same vertex winding.
    let mut synchronized_triangle_faces: Vec<TriangleFace> =
        Vec::with_capacity(original_triangle_faces.len());

    // Synchronize faces in all mesh geometry triangles
    while synchronized_triangle_faces.len() < original_triangle_faces.len() {
        // Start with the first untreated face in the list of the original mesh
        // geometry faces. The winding of this face also determines the winding
        // of the rest of the mesh geometry.
        let mut current_face_index: u32 = 0;
        while face_treatment_pattern[cast_usize(current_face_index)] {
            current_face_index += 1;
        }

        face_treatment_pattern[cast_usize(current_face_index)] = true;
        let mut current_test_edge: OrientedEdge =
            original_triangle_faces[cast_usize(current_face_index)].to_oriented_edges()[0];

        // Put the first edge-face couple to the stack
        edge_face_stack.push((current_test_edge, current_face_index));

        // Check and revert (if needed) faces in the stack as long as there is any
        while !edge_face_stack.is_empty() {
            // Get ready for the next iteration.
            let next_edge_face = edge_face_stack.pop().expect("Popping from an empty vector");
            current_test_edge = next_edge_face.0;
            current_face_index = next_edge_face.1;

            let current_face = original_triangle_faces[cast_usize(current_face_index)];
            // Use if contains reverted edge, if not, revert and use. It should be
            // safe to presume the face contains the edge one way or another because
            // it's given by the topology generator.
            let proper_current_face = if current_face.contains_oriented_edge(current_test_edge) {
                current_face
            } else {
                current_face.to_reverted()
            };
            synchronized_triangle_faces.push(proper_current_face);

            // Edge-to-index map for faster lookup
            let mut unoriented_edge_index_map: HashMap<UnorientedEdge, u32> = HashMap::new();
            for (unoriented_edge_index, unoriented_edge) in unoriented_edges.iter().enumerate() {
                unoriented_edge_index_map.insert(*unoriented_edge, cast_u32(unoriented_edge_index));
            }

            // Find the indices of edges of the current face in the list of edges,
            // from which the topology was created
            let face_unoriented_edges = proper_current_face.to_unoriented_edges();
            let face_unoriented_edge_indices =
                face_unoriented_edges.iter().map(|unoriented_edge| {
                    unoriented_edge_index_map
                        .get(unoriented_edge)
                        .expect("The current edge is not found in the edge collection")
                });

            // Convert into oriented edges so that the neighboring faces can check
            // for correct winding
            let face_oriented_edges: Vec<_> =
                face_unoriented_edges.iter().map(|u_e| u_e.0).collect();
            // For each face edge index
            for (i, face_unoriented_edge_index) in face_unoriented_edge_indices.enumerate() {
                // get the actual oriented edge
                let face_oriented_edge = face_oriented_edges[i];
                // and try to find it in the edge-to-face topology.
                if let Some(edge_in_faces) = edge_to_face_topology.get(&face_unoriented_edge_index)
                {
                    // If it exists, iterate the faces containing the edge
                    for face_index in edge_in_faces {
                        // and if it was not already added to the stack or even checked
                        if !face_treatment_pattern[cast_usize(*face_index)] {
                            // add it to the stack with the expected edge orientation
                            edge_face_stack.push((face_oriented_edge.to_reverted(), *face_index));
                            // and mark it treated.
                            face_treatment_pattern[cast_usize(*face_index)] = true;
                        }
                    }
                }
            }
        }
    }

    // Rebuild the mesh geometry with the new faces and original faces and normals.
    Geometry::from_triangle_faces_with_vertices_and_normals(
        synchronized_triangle_faces,
        geometry.vertices().to_vec(),
        geometry.normals().to_vec(),
    )
}

/// Reverts vertex and normal winding of all faces in the mesh geometry and
/// returns a reverted mesh geometry
pub fn revert_mesh_faces(geometry: &Geometry) -> Geometry {
    let reverted_faces = geometry.faces().iter().map(|face| match face {
        Face::Triangle(t_f) => t_f.to_reverted(),
    });
    Geometry::from_triangle_faces_with_vertices_and_normals(
        reverted_faces,
        geometry.vertices().to_vec(),
        geometry.normals().to_vec(),
    )
}

/// Weld similar (their distance is within the given tolerance) vertices into
/// one and reuse such vertices in connected faces.
///
/// Weld is used to actually connect faces (often resulting in a watertight
/// mesh), which are connected merely visually or to reduce number of vertices
/// in case when vertices are multiplied because vertices at the same
/// coordinates are not referenced by more faces but rather each face references
/// its own copy of the vertex.
///
/// Weld is one of the auto-fixes leading to a simplified, watertight or
/// true-to-its-genus mesh geometries.
pub fn weld(geometry: &Geometry, tolerance: f32) -> Geometry {
    // key = rounded vertex position with a tolerance (it's expected that the
    // same value will be shared by more close vertices)
    // value = actual positions of close vertices
    let mut vertex_proximity_map: HashMap<(i64, i64, i64), SmallVec<[usize; 8]>> = HashMap::new();
    for (current_vertex_index, vertex) in geometry.vertices().iter().enumerate() {
        let vertex_with_tolerance = (
            (vertex.x / tolerance).round() as i64,
            (vertex.y / tolerance).round() as i64,
            (vertex.z / tolerance).round() as i64,
        );

        vertex_proximity_map
            .entry(vertex_with_tolerance)
            .and_modify(|close_vertices| close_vertices.push(current_vertex_index))
            .or_insert_with(|| smallvec![current_vertex_index]);
    }

    // All vertices sorted into clusters of positionally close items. These will
    // be later averaged into a single vertex.
    let close_vertex_clusters = vertex_proximity_map.values();

    // key = original vertex index
    // value = new (averaged) vertex index It is expected that more keys will
    // share the same value; more original vertices will be replaced by a single
    // averaged vertex
    let mut old_new_vertex_map: HashMap<u32, u32> = HashMap::new();
    for (new_vertex_index, old_vertex_indices) in close_vertex_clusters.clone().enumerate() {
        for old_vertex_index in old_vertex_indices {
            old_new_vertex_map.insert(cast_u32(*old_vertex_index), cast_u32(new_vertex_index));
        }
    }

    // Vertices of the new geometry averaged from the clusters of original
    // vertices.
    let new_vertices = close_vertex_clusters.map(|old_vertex_indices| {
        old_vertex_indices
            .iter()
            .fold(Point3::origin(), |summed: Point3<f32>, old_vertex_index| {
                summed + geometry.vertices()[*old_vertex_index].coords
            })
            / old_vertex_indices.len() as f32
    });

    // New faces with renumbered vertex (and normal) indices. Some faces might
    // end up invalid (not referencing three distinct vertices). Those will be
    // removed as they don't affect the visual appearance of the mesh geometry.
    let new_faces = geometry
        .faces()
        .iter()
        .map(|old_face| match old_face {
            Face::Triangle(f) => Face::Triangle(TriangleFace::new(
                *old_new_vertex_map
                    .get(&f.vertices.0)
                    .expect("Referencing non-existent vertex"),
                *old_new_vertex_map
                    .get(&f.vertices.1)
                    .expect("Referencing non-existent vertex"),
                *old_new_vertex_map
                    .get(&f.vertices.2)
                    .expect("Referencing non-existent vertex"),
            )),
        })
        .filter(|new_face| match new_face {
            Face::Triangle(f) => f.vertices.0 != f.vertices.1 && f.vertices.0 != f.vertices.2,
        });

    // key = old vertex index
    // value = indices of all old normals being referenced by faces together
    // with the vertex
    //
    // The faces can reference vertices and normals in different ways. While the
    // vertices will be averaged using a straight-forward logic, it is unclear
    // which normals should be averaged to be matched with the new vertices.
    // Therefore it's important to collect all the normals associated with the
    // original vertices in clusters and averaging those.
    let mut old_vertex_normals_index_map: HashMap<u32, SmallVec<[u32; 8]>> = HashMap::new();
    for face in geometry.faces() {
        match face {
            Face::Triangle(f) => {
                let vertex_indices = [
                    (f.vertices.0, f.normals.0),
                    (f.vertices.1, f.normals.1),
                    (f.vertices.2, f.normals.2),
                ];
                for (vertex_index, normal_index) in &vertex_indices {
                    let associated_normals = old_vertex_normals_index_map
                        .entry(*vertex_index)
                        .or_insert_with(SmallVec::new);
                    if !associated_normals.contains(&normal_index) {
                        associated_normals.push(*normal_index);
                    }
                }
            }
        }
    }

    // Associate old normals to the new averaged vertices
    let mut new_vertex_old_normals_index_map: Vec<SmallVec<[u32; 8]>> =
        vec![SmallVec::new(); new_vertices.len()];
    for (old_vertex_index, old_normals_indices) in old_vertex_normals_index_map {
        let new_vertex_index = old_new_vertex_map
            .get(&old_vertex_index)
            .expect("The old vertex index not found in the old-new vertex map.");
        new_vertex_old_normals_index_map[cast_usize(*new_vertex_index)]
            .extend_from_slice(&old_normals_indices);
    }

    // Calculate an average normal for each new (averaged) vertex
    let new_normals: Vec<Vector3<f32>> = new_vertex_old_normals_index_map
        .iter()
        .map(|old_normals_indices| {
            old_normals_indices
                .iter()
                .fold(Vector3::zeros(), |avg, o_n_i| {
                    avg + geometry.normals()[cast_usize(*o_n_i)]
                })
                / old_normals_indices.len() as f32
        })
        .collect();

    Geometry::from_faces_with_vertices_and_normals(new_faces, new_vertices, new_normals)
}

/// Crawls the geometry to find continuous patches of geometry.
/// Returns a vector of new separated geometries.
pub fn separate_isolated_meshes(geometry: &Geometry) -> Vec<Geometry> {
    let face_to_face = mesh_topology_analysis::face_to_face_topology(geometry);
    let mut available_face_indices: HashSet<u32> = face_to_face.keys().cloned().collect();
    let mut patches: Vec<Geometry> = Vec::new();

    while let Some(first_face_index) = available_face_indices.iter().next() {
        let connected_indices = crawl_faces(*first_face_index, &face_to_face);

        patches.push(
            Geometry::from_faces_with_vertices_and_normals_remove_orphans(
                connected_indices
                    .iter()
                    .map(|face_index| geometry.faces()[cast_usize(*face_index)]),
                geometry.vertices().to_vec(),
                geometry.normals().to_vec(),
            ),
        );

        for c in &connected_indices {
            available_face_indices.remove(c);
        }
    }

    patches
}

fn crawl_faces(
    start_face_index: u32,
    face_to_face: &HashMap<u32, SmallVec<[u32; 8]>>,
) -> HashSet<u32> {
    let mut index_stack = vec![start_face_index];
    index_stack.push(start_face_index);

    let mut connected_face_indices = HashSet::new();

    while let Some(current_face_index) = index_stack.pop() {
        if connected_face_indices.insert(current_face_index) {
            for neighbor in &face_to_face[&current_face_index] {
                index_stack.push(neighbor.clone());
            }
        }
    }

    connected_face_indices.shrink_to_fit();

    connected_face_indices
}

/// Joins two mesh geometries into one.
///
/// Concatenates vertex and normal slices, while keeping the first mesh
/// geometry's element indices intact and second geometry's indices offset by
/// the length of the respective elements. Reuses first mesh geometry's faces
/// and recomputes the second mesh geometry's faces to match new indices of its
/// elements.
pub fn join_meshes(first_geometry: &Geometry, second_geometry: &Geometry) -> Geometry {
    let vertex_offset = first_geometry.vertices().len();
    let mut vertices: Vec<Point3<f32>> =
        Vec::with_capacity(vertex_offset + second_geometry.vertices().len());
    vertices.extend_from_slice(first_geometry.vertices());
    vertices.extend_from_slice(second_geometry.vertices());

    let normal_offset = first_geometry.normals().len();
    let mut normals: Vec<Vector3<f32>> =
        Vec::with_capacity(normal_offset + second_geometry.normals().len());
    normals.extend_from_slice(first_geometry.normals());
    normals.extend_from_slice(second_geometry.normals());

    let mut faces: Vec<Face> =
        Vec::with_capacity(first_geometry.faces().len() + second_geometry.faces().len());
    faces.extend_from_slice(first_geometry.faces());
    let vertex_offset_u32 = cast_u32(vertex_offset);
    let normal_offset_u32 = cast_u32(normal_offset);
    for face in second_geometry.faces() {
        match face {
            Face::Triangle(f) => faces.push(Face::Triangle(TriangleFace::new_separate(
                f.vertices.0 + vertex_offset_u32,
                f.vertices.1 + vertex_offset_u32,
                f.vertices.2 + vertex_offset_u32,
                f.normals.0 + normal_offset_u32,
                f.normals.1 + normal_offset_u32,
                f.normals.2 + normal_offset_u32,
            ))),
        }
    }

    Geometry::from_faces_with_vertices_and_normals(faces, vertices, normals)
}

#[cfg(test)]
mod tests {
    use nalgebra::base::Vector3;
    use nalgebra::geometry::Point3;

    use crate::geometry::{self, Geometry, TriangleFace};
    use crate::mesh_analysis;

    use super::*;

    fn v(x: f32, y: f32, z: f32, translation: [f32; 3], scale: f32) -> Point3<f32> {
        Point3::new(
            scale * x + translation[0],
            scale * y + translation[1],
            scale * z + translation[2],
        )
    }

    fn n(x: f32, y: f32, z: f32) -> Vector3<f32> {
        Vector3::new(x, y, z)
    }

    fn tessellated_triangle_geometry_after_welding() -> Geometry {
        let vertices = vec![
            Point3::new(-2.0, -2.0, 0.0),
            Point3::new(0.0, -2.0, 0.0),
            Point3::new(2.0, -2.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];

        let vertex_normals = vec![
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
        ];

        let faces = vec![
            TriangleFace::new_separate(0, 1, 3, 0, 1, 3),
            TriangleFace::new_separate(1, 4, 3, 1, 4, 3),
            TriangleFace::new_separate(1, 2, 4, 1, 2, 4),
            TriangleFace::new_separate(3, 4, 5, 3, 4, 5),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    fn tessellated_triangle_geometry_for_welding() -> Geometry {
        let vertices = vec![
            Point3::new(-2.0, -2.0, 0.0), //0, 0
            Point3::new(0.0, -2.0, 0.0),  //1, 1
            Point3::new(-1.0, 0.0, 0.0),  //3, 2
            Point3::new(0.0, -2.0, 0.0),  //1, 3
            Point3::new(2.0, -2.0, 0.0),  //2, 4
            Point3::new(1.0, 0.0, 0.0),   //4, 5
            Point3::new(1.0, 0.0, 0.0),   //4, 6
            Point3::new(-1.0, 0.0, 0.0),  //3, 7
            Point3::new(0.0, -2.0, 0.0),  //1, 8
            Point3::new(-1.0, 0.0, 0.0),  //3, 9
            Point3::new(1.0, 0.0, 0.0),   //4, 10
            Point3::new(0.0, 2.0, 0.0),   //5, 11
        ];

        let vertex_normals = vec![n(0.0, 0.0, 1.0)];

        let faces = vec![
            TriangleFace::new_separate(0, 1, 2, 0, 0, 0),
            TriangleFace::new_separate(3, 4, 5, 0, 0, 0),
            TriangleFace::new_separate(6, 7, 8, 0, 0, 0),
            TriangleFace::new_separate(9, 10, 11, 0, 0, 0),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    fn tessellated_triangle_geometry() -> Geometry {
        let vertices = vec![
            Point3::new(-2.0, -2.0, 0.0),
            Point3::new(0.0, -2.0, 0.0),
            Point3::new(2.0, -2.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ];

        let vertex_normals = vec![n(0.0, 0.0, 1.0)];

        let faces = vec![
            TriangleFace::new_separate(0, 3, 1, 0, 0, 0),
            TriangleFace::new_separate(1, 3, 4, 0, 0, 0),
            TriangleFace::new_separate(1, 4, 2, 0, 0, 0),
            TriangleFace::new_separate(3, 5, 4, 0, 0, 0),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    fn empty_geometry() -> Geometry {
        let vertices = vec![];

        let vertex_normals = vec![];

        let faces = vec![];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    fn tessellated_triangle_with_island_geometry() -> Geometry {
        let vertices = vec![
            Point3::new(-2.0, -2.0, 0.0),
            Point3::new(0.0, -2.0, 0.0),
            Point3::new(2.0, -2.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
            Point3::new(-1.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 2.0, 1.0),
        ];

        let vertex_normals = vec![n(0.0, 0.0, 1.0), n(0.0, 0.0, 1.0)];

        let faces = vec![
            TriangleFace::new_separate(0, 3, 1, 0, 0, 0),
            TriangleFace::new_separate(1, 3, 4, 0, 0, 0),
            TriangleFace::new_separate(1, 4, 2, 0, 0, 0),
            TriangleFace::new_separate(3, 5, 4, 0, 0, 0),
            TriangleFace::new_separate(6, 7, 8, 1, 1, 1),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    fn tessellated_triangle_with_island_geometry_with_flipped_face() -> Geometry {
        let vertices = vec![
            Point3::new(-2.0, -2.0, 0.0),
            Point3::new(0.0, -2.0, 0.0),
            Point3::new(2.0, -2.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
            Point3::new(-1.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 2.0, 1.0),
        ];

        let vertex_normals = vec![n(0.0, 0.0, 1.0), n(0.0, 0.0, 1.0)];

        let faces = vec![
            TriangleFace::new_separate(0, 3, 1, 0, 0, 0),
            TriangleFace::new_separate(1, 3, 4, 0, 0, 0),
            TriangleFace::new_separate(2, 4, 1, 0, 0, 0),
            TriangleFace::new_separate(3, 5, 4, 0, 0, 0),
            TriangleFace::new_separate(6, 7, 8, 1, 1, 1),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    fn triangular_island_geometry() -> Geometry {
        let vertices = vec![
            Point3::new(-1.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 2.0, 1.0),
        ];

        let vertex_normals = vec![n(0.0, 0.0, 1.0)];

        let faces = vec![TriangleFace::new_separate(0, 1, 2, 0, 0, 0)];

        Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertices, vertex_normals)
    }

    pub fn cube_sharp_same_len(position: [f32; 3], scale: f32) -> Geometry {
        let vertex_positions = vec![
            // back
            v(-1.0, 1.0, -1.0, position, scale), //0
            v(-1.0, 1.0, 1.0, position, scale),  //1
            v(1.0, 1.0, 1.0, position, scale),   //2
            v(1.0, 1.0, -1.0, position, scale),  //3
            // front
            v(-1.0, -1.0, -1.0, position, scale), //4
            v(1.0, -1.0, -1.0, position, scale),  //5
            v(1.0, -1.0, 1.0, position, scale),   //6
            v(-1.0, -1.0, 1.0, position, scale),  //7
            // top
            v(-1.0, 1.0, 1.0, position, scale),  //8
            v(-1.0, -1.0, 1.0, position, scale), //9
            v(1.0, -1.0, 1.0, position, scale),  //10
            v(1.0, 1.0, 1.0, position, scale),   //11
            // bottom
            v(-1.0, 1.0, -1.0, position, scale),  //12
            v(1.0, 1.0, -1.0, position, scale),   //13
            v(1.0, -1.0, -1.0, position, scale),  //14
            v(-1.0, -1.0, -1.0, position, scale), //15
            // right
            v(1.0, 1.0, -1.0, position, scale),  //16
            v(1.0, 1.0, 1.0, position, scale),   //17
            v(1.0, -1.0, 1.0, position, scale),  //18
            v(1.0, -1.0, -1.0, position, scale), //19
            // left
            v(-1.0, 1.0, -1.0, position, scale),  //20
            v(-1.0, -1.0, -1.0, position, scale), //21
            v(-1.0, -1.0, 1.0, position, scale),  //22
            v(-1.0, 1.0, 1.0, position, scale),   //23
        ];

        let vertex_normals = vec![
            // back
            n(0.0, 1.0, 0.0),
            n(0.0, 1.0, 0.0),
            n(0.0, 1.0, 0.0),
            n(0.0, 1.0, 0.0),
            // front
            n(0.0, -1.0, 0.0),
            n(0.0, -1.0, 0.0),
            n(0.0, -1.0, 0.0),
            n(0.0, -1.0, 0.0),
            // top
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            n(0.0, 0.0, 1.0),
            // bottom
            n(0.0, 0.0, -1.0),
            n(0.0, 0.0, -1.0),
            n(0.0, 0.0, -1.0),
            n(0.0, 0.0, -1.0),
            // right
            n(1.0, 0.0, 0.0),
            n(1.0, 0.0, 0.0),
            n(1.0, 0.0, 0.0),
            n(1.0, 0.0, 0.0),
            // left
            n(-1.0, 0.0, 0.0),
            n(-1.0, 0.0, 0.0),
            n(-1.0, 0.0, 0.0),
            n(-1.0, 0.0, 0.0),
        ];

        let faces = vec![
            // back
            TriangleFace::new(0, 1, 2),
            TriangleFace::new(2, 3, 0),
            // front
            TriangleFace::new(4, 5, 6),
            TriangleFace::new(6, 7, 4),
            // top
            TriangleFace::new(8, 9, 10),
            TriangleFace::new(10, 11, 8),
            // bottom
            TriangleFace::new(12, 13, 14),
            TriangleFace::new(14, 15, 12),
            // right
            TriangleFace::new(16, 17, 18),
            TriangleFace::new(18, 19, 16),
            // left
            TriangleFace::new(20, 21, 22),
            TriangleFace::new(22, 23, 20),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(
            faces,
            vertex_positions,
            vertex_normals,
        )
    }

    pub fn cube_smooth_var_len_like_after_welding(position: [f32; 3], scale: f32) -> Geometry {
        let vertex_positions = vec![
            // back
            v(-1.0, 1.0, -1.0, position, scale),
            v(-1.0, 1.0, 1.0, position, scale),
            v(1.0, 1.0, 1.0, position, scale),
            v(1.0, 1.0, -1.0, position, scale),
            // front
            v(-1.0, -1.0, -1.0, position, scale),
            v(1.0, -1.0, -1.0, position, scale),
            v(1.0, -1.0, 1.0, position, scale),
            v(-1.0, -1.0, 1.0, position, scale),
        ];

        let vertex_normals = vec![
            n(-1.0 / 3.0, 1.0 / 3.0, -1.0 / 3.0),
            n(-1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
            n(1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
            n(1.0 / 3.0, 1.0 / 3.0, -1.0 / 3.0),
            // front
            n(-1.0 / 3.0, -1.0 / 3.0, -1.0 / 3.0),
            n(1.0 / 3.0, -1.0 / 3.0, -1.0 / 3.0),
            n(1.0 / 3.0, -1.0 / 3.0, 1.0 / 3.0),
            n(-1.0 / 3.0, -1.0 / 3.0, 1.0 / 3.0),
        ];

        let faces = vec![
            // back
            TriangleFace::new(0, 1, 2),
            TriangleFace::new(2, 3, 0),
            // front
            TriangleFace::new(4, 5, 6),
            TriangleFace::new(6, 7, 4),
            // top
            TriangleFace::new(7, 6, 1),
            TriangleFace::new(2, 1, 6),
            // bottom
            TriangleFace::new(5, 0, 3),
            TriangleFace::new(0, 5, 4),
            // right
            TriangleFace::new(6, 3, 2),
            TriangleFace::new(3, 6, 5),
            // left
            TriangleFace::new(4, 7, 0),
            TriangleFace::new(1, 0, 7),
        ];

        Geometry::from_triangle_faces_with_vertices_and_normals(
            faces,
            vertex_positions,
            vertex_normals,
        )
    }

    #[test]
    fn test_separate_isolated_meshes_returns_similar_for_tessellated_triangle() {
        let geometry = tessellated_triangle_geometry();

        let calculated_geometries = separate_isolated_meshes(&geometry);

        assert_eq!(calculated_geometries.len(), 1);

        assert!(mesh_analysis::are_similar(
            &calculated_geometries[0],
            &geometry
        ));
    }

    #[test]
    fn test_separate_isolated_meshes_returns_similar_for_cube() {
        let geometry = geometry::cube_sharp_var_len([0.0, 0.0, 0.0], 1.0);

        let calculated_geometries = separate_isolated_meshes(&geometry);

        assert_eq!(calculated_geometries.len(), 1);
        assert!(mesh_analysis::are_similar(
            &geometry,
            &calculated_geometries[0]
        ));
    }

    #[test]
    fn test_separate_isolated_meshes_returns_similar_for_tessellated_triangle_with_island() {
        let geometry = tessellated_triangle_with_island_geometry();
        let geometry_triangle_correct = tessellated_triangle_geometry();
        let geometry_island_correct = triangular_island_geometry();

        let calculated_geometries = separate_isolated_meshes(&geometry);

        assert_eq!(calculated_geometries.len(), 2);

        if mesh_analysis::are_similar(&calculated_geometries[0], &geometry_triangle_correct) {
            assert!(mesh_analysis::are_similar(
                &calculated_geometries[1],
                &geometry_island_correct
            ));
        } else {
            assert!(mesh_analysis::are_similar(
                &calculated_geometries[1],
                &geometry_triangle_correct
            ));
            assert!(mesh_analysis::are_similar(
                &calculated_geometries[0],
                &geometry_island_correct
            ));
        }
    }

    #[test]
    fn test_mesh_tools_revert_mesh_faces() {
        let test_geometry = geometry::cube_sharp_var_len([0.0, 0.0, 0.0], 1.0);

        let calculated_geometry = revert_mesh_faces(&test_geometry);

        assert_eq!(
            test_geometry.faces().len(),
            calculated_geometry.faces().len()
        );

        assert!(calculated_geometry.faces().iter().all(|face| match face {
            Face::Triangle(f) => test_geometry
                .faces()
                .iter()
                .any(|other_face| match other_face {
                    Face::Triangle(o_f) => o_f.is_reverted(*f),
                }),
        }));
    }

    #[test]
    fn test_mesh_tools_synchronize_mesh_winding() {
        let test_geometry = tessellated_triangle_with_island_geometry_with_flipped_face();
        let test_geometry_correct = tessellated_triangle_with_island_geometry();

        let unoriented_edges: Vec<_> = test_geometry.unoriented_edges_iter().collect();
        let edge_to_face =
            mesh_topology_analysis::edge_to_face_topology(&test_geometry, &unoriented_edges);

        let calculated_geometry =
            synchronize_mesh_winding(&test_geometry, &unoriented_edges, &edge_to_face);

        assert_eq!(test_geometry_correct, calculated_geometry);
    }

    #[test]
    fn test_weld_tesselated_triangle() {
        let geometry = tessellated_triangle_geometry_for_welding();
        let geometry_after_welding_correct = tessellated_triangle_geometry_after_welding();

        let geometry_after_welding = weld(&geometry, 0.1);

        assert!(mesh_analysis::are_similar(
            &geometry_after_welding_correct,
            &geometry_after_welding
        ));
    }

    #[test]
    fn test_weld_cube_sharp_same_len() {
        let geometry = cube_sharp_same_len([0.0, 0.0, 0.0], 1.0);
        let geometry_after_welding_correct =
            cube_smooth_var_len_like_after_welding([0.0, 0.0, 0.0], 1.0);

        let geometry_after_welding = weld(&geometry, 0.1);

        assert!(mesh_analysis::are_similar(
            &geometry_after_welding_correct,
            &geometry_after_welding
        ));
    }

    #[test]
    fn test_join_meshes_tessellated_triangle_and_empty() {
        let tessellated_triangle_geometry = tessellated_triangle_geometry();
        let empty_geometry = empty_geometry();

        let calculated_geometry = join_meshes(&tessellated_triangle_geometry, &empty_geometry);

        assert_eq!(&tessellated_triangle_geometry, &calculated_geometry);
    }

    #[test]
    fn test_join_meshes_tessellated_empty_and_triangle() {
        let empty_geometry = empty_geometry();
        let tessellated_triangle_geometry = tessellated_triangle_geometry();

        let calculated_geometry = join_meshes(&empty_geometry, &tessellated_triangle_geometry);

        assert_eq!(&tessellated_triangle_geometry, &calculated_geometry);
    }

    #[test]
    fn test_join_meshes_returns_tessellated_triangle_with_island() {
        let tessellated_triangle = tessellated_triangle_geometry();
        let triangular_island = triangular_island_geometry();

        let geometry_correct = tessellated_triangle_with_island_geometry();

        let calculated_geometry = join_meshes(&tessellated_triangle, &triangular_island);

        assert_eq!(&geometry_correct, &calculated_geometry);
    }
}
