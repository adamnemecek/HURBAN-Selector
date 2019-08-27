use nalgebra as na;
use nalgebra::base::Vector3;
use nalgebra::geometry::Point3;

use crate::convert::cast_u32;

/// Geometric data containing multiple possibly _variable-length_
/// lists of geometric data, such as vertices and normals, and faces -
/// a single list containing the index topology that describes the
/// structure of data in those lists.
///
/// Currently only `Face::Triangle` is supported. It binds vertices
/// and normals in triangular faces. `Face::Triangle` is always
/// ensured to have counter-clockwise winding. Quad or polygonal faces
/// are not supported currently, but might be in the future.
///
/// The geometry data lives in right-handed coordinate space with the
/// XY plance being the ground and Z axis growing upwards.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    faces: Vec<Face>,
    vertices: Vec<Point3<f32>>,
    normals: Option<Vec<Vector3<f32>>>,
}

impl Geometry {
    /// Create new triangle face geometry from provided faces and vertices.
    ///
    /// # Panics
    /// Panics if faces refer to out-of-bounds vertices.
    pub fn from_triangle_faces_with_vertices(
        faces: Vec<TriangleFace>,
        vertices: Vec<Point3<f32>>,
    ) -> Self {
        // FIXME: orphan removal

        let vertices_range = 0..cast_u32(vertices.len());
        for face in &faces {
            let v = face.vertices;
            assert!(
                vertices_range.contains(&v.0),
                "Faces reference out of bounds data"
            );
            assert!(
                vertices_range.contains(&v.1),
                "Faces reference out of bounds data"
            );
            assert!(
                vertices_range.contains(&v.2),
                "Faces reference out of bounds data"
            );
        }

        Self {
            faces: faces.into_iter().map(Face::Triangle).collect(),
            vertices,
            normals: None,
        }
    }

    /// Create new triangle face geometry from provided faces,
    /// vertices, and normals.
    ///
    /// # Panics
    /// Panics if faces refer to out-of-bounds vertices or normals.
    pub fn from_triangle_faces_with_vertices_and_normals(
        faces: Vec<TriangleFace>,
        vertices: Vec<Point3<f32>>,
        normals: Vec<Vector3<f32>>,
    ) -> Self {
        // FIXME: orphan removal

        let vertices_range = 0..cast_u32(vertices.len());
        let normals_range = 0..cast_u32(normals.len());
        for face in &faces {
            let v = face.vertices;
            let n = face.normals.expect("Normals must be present in faces");
            assert!(
                vertices_range.contains(&v.0),
                "Faces reference out of bounds data"
            );
            assert!(
                vertices_range.contains(&v.1),
                "Faces reference out of bounds data"
            );
            assert!(
                vertices_range.contains(&v.2),
                "Faces reference out of bounds data"
            );
            assert!(
                normals_range.contains(&n.0),
                "Faces reference out of bounds data"
            );
            assert!(
                normals_range.contains(&n.1),
                "Faces reference out of bounds data"
            );
            assert!(
                normals_range.contains(&n.2),
                "Faces reference out of bounds data"
            );
        }

        Self {
            faces: faces.into_iter().map(Face::Triangle).collect(),
            vertices,
            normals: Some(normals),
        }
    }

    /// Return a view of all triangle faces in this geometry. Skip all
    /// other types of faces.
    pub fn triangle_faces_iter<'a>(&'a self) -> impl Iterator<Item = TriangleFace> + 'a {
        self.faces.iter().copied().map(|index| match index {
            Face::Triangle(f) => f,
        })
    }

    /// Return count of all triangle faces in this geometry. Skip all
    /// other types of faces.
    pub fn triangle_faces_len(&self) -> usize {
        self.faces
            .iter()
            .filter(|index| match index {
                Face::Triangle(_) => true,
            })
            .count()
    }

    pub fn vertices(&self) -> &[Point3<f32>] {
        &self.vertices
    }

    pub fn normals(&self) -> Option<&[Vector3<f32>]> {
        self.normals.as_ref().map(Vec::as_slice)
    }
}

/// A geometry index. Describes topology of geometry data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Triangle(TriangleFace),
}

/// A triangular face. Contains indices to other geometry data, such
/// as vertices and normals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriangleFace {
    pub vertices: (u32, u32, u32),
    pub normals: Option<(u32, u32, u32)>,
    // tex_coords
}

pub fn plane_same_len(position: [f32; 3], scale: f32) -> Geometry {
    #[rustfmt::skip]
    let vertex_positions = vec![
        v(-1.0, -1.0,  0.0, position, scale),
        v( 1.0, -1.0,  0.0, position, scale),
        v( 1.0,  1.0,  0.0, position, scale),
        v( 1.0,  1.0,  0.0, position, scale),
        v(-1.0,  1.0,  0.0, position, scale),
        v(-1.0, -1.0,  0.0, position, scale),
    ];

    #[rustfmt::skip]
    let vertex_normals = vec![
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
    ];

    #[rustfmt::skip]
    let faces = vec![
        tf_vn(0, 1, 2),
        tf_vn(3, 4, 5),
    ];

    Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertex_positions, vertex_normals)
}

pub fn plane_var_len(position: [f32; 3], scale: f32) -> Geometry {
    #[rustfmt::skip]
    let vertex_positions = vec![
        v(-1.0, -1.0,  0.0, position, scale),
        v( 1.0, -1.0,  0.0, position, scale),
        v( 1.0,  1.0,  0.0, position, scale),
        v( 1.0,  1.0,  0.0, position, scale),
        v(-1.0,  1.0,  0.0, position, scale),
        v(-1.0, -1.0,  0.0, position, scale),
    ];

    #[rustfmt::skip]
    let vertex_normals = vec![
        n( 0.0,  0.0,  1.0),
    ];

    #[rustfmt::skip]
    let faces = vec![
        tf_vn_separate(0, 1, 2, 0, 0, 0),
        tf_vn_separate(3, 4, 5, 0, 0, 0),
    ];

    Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertex_positions, vertex_normals)
}

pub fn cube_same_len(position: [f32; 3], scale: f32) -> Geometry {
    #[rustfmt::skip]
    let vertex_positions = vec![
        // back
        v(-1.0,  1.0, -1.0, position, scale),
        v(-1.0,  1.0,  1.0, position, scale),
        v( 1.0,  1.0,  1.0, position, scale),
        v( 1.0,  1.0, -1.0, position, scale),
        // front
        v(-1.0, -1.0, -1.0, position, scale),
        v( 1.0, -1.0, -1.0, position, scale),
        v( 1.0, -1.0,  1.0, position, scale),
        v(-1.0, -1.0,  1.0, position, scale),
    ];

    // FIXME: make const once float arithmetic is stabilized in const fns
    // let sqrt_3 = 3.0f32.sqrt();
    let frac_1_sqrt_3 = 1.0 / 3.0_f32.sqrt();

    #[rustfmt::skip]
    let vertex_normals = vec![
        // back
        n(-frac_1_sqrt_3,  frac_1_sqrt_3, -frac_1_sqrt_3),
        n(-frac_1_sqrt_3,  frac_1_sqrt_3,  frac_1_sqrt_3),
        n( frac_1_sqrt_3,  frac_1_sqrt_3,  frac_1_sqrt_3),
        n( frac_1_sqrt_3,  frac_1_sqrt_3, -frac_1_sqrt_3),
        // front
        n(-frac_1_sqrt_3, -frac_1_sqrt_3, -frac_1_sqrt_3),
        n( frac_1_sqrt_3, -frac_1_sqrt_3, -frac_1_sqrt_3),
        n( frac_1_sqrt_3, -frac_1_sqrt_3,  frac_1_sqrt_3),
        n(-frac_1_sqrt_3, -frac_1_sqrt_3,  frac_1_sqrt_3),
    ];

    #[rustfmt::skip]
    let faces = vec![
        // back
        tf_vn(0, 1, 2),
        tf_vn(2, 3, 0),
        // front
        tf_vn(4, 5, 6),
        tf_vn(6, 7, 4),
        // top
        tf_vn(7, 6, 2),
        tf_vn(2, 1, 7),
        // bottom
        tf_vn(4, 0, 3),
        tf_vn(3, 5, 4),
        // right
        tf_vn(5, 3, 2),
        tf_vn(2, 6, 5),
        // left
        tf_vn(4, 7, 1),
        tf_vn(1, 0, 4),
    ];

    Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertex_positions, vertex_normals)
}

pub fn uv_cube_same_len(position: [f32; 3], scale: f32) -> Geometry {
    #[rustfmt::skip]
    let vertex_positions = vec![
        // back
        v(-1.0,  1.0, -1.0, position, scale),
        v(-1.0,  1.0,  1.0, position, scale),
        v( 1.0,  1.0,  1.0, position, scale),
        v( 1.0,  1.0, -1.0, position, scale),
        // front
        v(-1.0, -1.0, -1.0, position, scale),
        v( 1.0, -1.0, -1.0, position, scale),
        v( 1.0, -1.0,  1.0, position, scale),
        v(-1.0, -1.0,  1.0, position, scale),
        // top
        v(-1.0,  1.0,  1.0, position, scale),
        v(-1.0, -1.0,  1.0, position, scale),
        v( 1.0, -1.0,  1.0, position, scale),
        v( 1.0,  1.0,  1.0, position, scale),
        // bottom
        v(-1.0,  1.0, -1.0, position, scale),
        v( 1.0,  1.0, -1.0, position, scale),
        v( 1.0, -1.0, -1.0, position, scale),
        v(-1.0, -1.0, -1.0, position, scale),
        // right
        v( 1.0,  1.0, -1.0, position, scale),
        v( 1.0,  1.0,  1.0, position, scale),
        v( 1.0, -1.0,  1.0, position, scale),
        v( 1.0, -1.0, -1.0, position, scale),
        // left
        v(-1.0,  1.0, -1.0, position, scale),
        v(-1.0, -1.0, -1.0, position, scale),
        v(-1.0, -1.0,  1.0, position, scale),
        v(-1.0,  1.0,  1.0, position, scale),
    ];

    #[rustfmt::skip]
    let vertex_normals = vec![
        // back
        n( 0.0,  1.0,  0.0),
        n( 0.0,  1.0,  0.0),
        n( 0.0,  1.0,  0.0),
        n( 0.0,  1.0,  0.0),
        // front
        n( 0.0, -1.0,  0.0),
        n( 0.0, -1.0,  0.0),
        n( 0.0, -1.0,  0.0),
        n( 0.0, -1.0,  0.0),
        // top
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        n( 0.0,  0.0,  1.0),
        // bottom
        n( 0.0,  0.0, -1.0),
        n( 0.0,  0.0, -1.0),
        n( 0.0,  0.0, -1.0),
        n( 0.0,  0.0, -1.0),
        // right
        n( 1.0,  0.0,  0.0),
        n( 1.0,  0.0,  0.0),
        n( 1.0,  0.0,  0.0),
        n( 1.0,  0.0,  0.0),
        // left
        n(-1.0,  0.0,  0.0),
        n(-1.0,  0.0,  0.0),
        n(-1.0,  0.0,  0.0),
        n(-1.0,  0.0,  0.0),
    ];

    #[rustfmt::skip]
    let faces = vec![
        // back
        tf_vn(0, 1, 2),
        tf_vn(2, 3, 0),
        // front
        tf_vn(4, 5, 6),
        tf_vn(6, 7, 4),
        // top
        tf_vn(8, 9, 10),
        tf_vn(10, 11, 8),
        // bottom
        tf_vn(12, 13, 14),
        tf_vn(14, 15, 12),
        // right
        tf_vn(16, 17, 18),
        tf_vn(18, 19, 16),
        // left
        tf_vn(20, 21, 22),
        tf_vn(22, 23, 20),
    ];

    Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertex_positions, vertex_normals)
}

pub fn uv_cube_var_len(position: [f32; 3], scale: f32) -> Geometry {
    #[rustfmt::skip]
    let vertex_positions = vec![
        // back
        v(-1.0,  1.0, -1.0, position, scale),
        v(-1.0,  1.0,  1.0, position, scale),
        v( 1.0,  1.0,  1.0, position, scale),
        v( 1.0,  1.0, -1.0, position, scale),
        // front
        v(-1.0, -1.0, -1.0, position, scale),
        v( 1.0, -1.0, -1.0, position, scale),
        v( 1.0, -1.0,  1.0, position, scale),
        v(-1.0, -1.0,  1.0, position, scale),
    ];

    #[rustfmt::skip]
    let vertex_normals = vec![
        // back
        n( 0.0,  1.0,  0.0),
        // front
        n( 0.0, -1.0,  0.0),
        // top
        n( 0.0,  0.0,  1.0),
        // bottom
        n( 0.0,  0.0, -1.0),
        // right
        n( 1.0,  0.0,  0.0),
        // left
        n(-1.0,  0.0,  0.0),
    ];

    #[rustfmt::skip]
    let faces = vec![
        // back
        tf_vn_separate(0, 1, 2, 0, 0, 0),
        tf_vn_separate(2, 3, 0, 0, 0, 0),
        // front
        tf_vn_separate(4, 5, 6, 1, 1, 1),
        tf_vn_separate(6, 7, 4, 1, 1, 1),
        // top
        tf_vn_separate(7, 6, 2, 2, 2, 2),
        tf_vn_separate(2, 1, 7, 2, 2, 2),
        // bottom
        tf_vn_separate(4, 0, 3, 3, 3, 3),
        tf_vn_separate(3, 5, 4, 3, 3, 3),
        // right
        tf_vn_separate(5, 3, 2, 4, 4, 4),
        tf_vn_separate(2, 6, 5, 4, 4, 4),
        // left
        tf_vn_separate(4, 7, 1, 5, 5, 5),
        tf_vn_separate(1, 0, 4, 5, 5, 5),
    ];

    Geometry::from_triangle_faces_with_vertices_and_normals(faces, vertex_positions, vertex_normals)
}

pub fn compute_bounding_sphere(geometries: &[Geometry]) -> (Point3<f32>, f32) {
    let centroid = compute_centroid(geometries);
    let mut max_distance = 0.0;

    for geometry in geometries {
        for vertex in &geometry.vertices {
            // Can't use `distance_squared` for values 0..1

            // FIXME: @Optimization Benchmark this against a 0..1 vs
            // 1..inf branching version using distance_squared for 1..inf
            let distance = na::distance(&centroid, vertex);
            if distance > max_distance {
                max_distance = distance;
            }
        }
    }

    (centroid, max_distance)
}

pub fn compute_centroid(geometries: &[Geometry]) -> Point3<f32> {
    let mut vertex_count = 0;
    let mut centroid = Point3::origin();
    for geometry in geometries {
        vertex_count += geometry.vertices.len();
        for vertex in &geometry.vertices {
            let v = vertex - Point3::origin();
            centroid += v;
        }
    }

    centroid / (vertex_count as f32)
}

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

fn tf_vn(i1: u32, i2: u32, i3: u32) -> TriangleFace {
    TriangleFace {
        vertices: (i1, i2, i3),
        normals: Some((i1, i2, i3)),
    }
}

fn tf_vn_separate(v1: u32, v2: u32, v3: u32, n1: u32, n2: u32, n3: u32) -> TriangleFace {
    TriangleFace {
        vertices: (v1, v2, v3),
        normals: Some((n1, n2, n3)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> (Vec<TriangleFace>, Vec<Point3<f32>>) {
        #[rustfmt::skip]
        let vertices = vec![
            v(-1.0, -1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
            v( 1.0, -1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
            v( 1.0,  1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
            v(-1.0,  1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
        ];

        #[rustfmt::skip]
        let faces = vec![
            tf_v(0, 1, 2),
            tf_v(2, 3, 0),
        ];

        (faces, vertices)
    }

    fn quad_with_normals() -> (Vec<TriangleFace>, Vec<Point3<f32>>, Vec<Vector3<f32>>) {
        #[rustfmt::skip]
        let vertices = vec![
            v(-1.0, -1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
            v( 1.0, -1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
            v( 1.0,  1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
            v(-1.0,  1.0,  0.0, [0.0, 0.0, 0.0], 1.0),
        ];

        #[rustfmt::skip]
        let normals = vec![
            n( 0.0,  0.0,  1.0),
            n( 0.0,  0.0,  1.0),
            n( 0.0,  0.0,  1.0),
            n( 0.0,  0.0,  1.0),
        ];

        #[rustfmt::skip]
        let faces = vec![
            tf_vn(0, 1, 2),
            tf_vn(2, 3, 0),
        ];

        (faces, vertices, normals)
    }

    fn tf_v(a: u32, b: u32, c: u32) -> TriangleFace {
        TriangleFace {
            vertices: (a, b, c),
            normals: None,
        }
    }

    #[test]
    fn test_geometry_from_triangle_faces_with_vertices() {
        let (faces, vertices) = quad();
        let geometry = Geometry::from_triangle_faces_with_vertices(faces.clone(), vertices.clone());
        let geometry_faces: Vec<_> = geometry.triangle_faces_iter().collect();

        assert_eq!(vertices.as_slice(), geometry.vertices());
        assert_eq!(faces.as_slice(), geometry_faces.as_slice());
    }

    #[test]
    #[should_panic(expected = "Faces reference out of bounds data")]
    fn test_geometry_from_triangle_faces_with_vertices_bounds_check() {
        let (_, vertices) = quad();
        #[rustfmt::skip]
        let faces = vec![
            tf_v(0, 1, 2),
            tf_v(2, 3, 4),
        ];

        let _geometry = Geometry::from_triangle_faces_with_vertices(faces, vertices);
    }

    #[test]
    fn test_geometry_from_triangle_faces_with_vertices_and_normals() {
        let (faces, vertices, normals) = quad_with_normals();
        let geometry = Geometry::from_triangle_faces_with_vertices_and_normals(
            faces.clone(),
            vertices.clone(),
            normals.clone(),
        );
        let geometry_faces: Vec<_> = geometry.triangle_faces_iter().collect();

        assert_eq!(vertices.as_slice(), geometry.vertices());
        assert_eq!(normals.as_slice(), geometry.normals().unwrap());
        assert_eq!(faces.as_slice(), geometry_faces.as_slice());
    }

    #[test]
    #[should_panic(expected = "Faces reference out of bounds data")]
    fn test_geometry_from_triangle_faces_with_vertices_and_normals_bounds_check() {
        let (_, vertices, normals) = quad_with_normals();
        #[rustfmt::skip]
        let faces = vec![
            tf_vn(0, 1, 2),
            tf_vn(2, 3, 4),
        ];

        let _geometry = Geometry::from_triangle_faces_with_vertices_and_normals(
            faces.clone(),
            vertices.clone(),
            normals.clone(),
        );
    }
}
