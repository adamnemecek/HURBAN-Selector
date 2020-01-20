use std::error;
use std::f32;
use std::fmt;
use std::sync::Arc;

use nalgebra::Vector3;

use crate::interpreter::{
    BooleanParamRefinement, Float3ParamRefinement, FloatParamRefinement, Func, FuncError,
    FuncFlags, FuncInfo, LogMessage, ParamInfo, ParamRefinement, Ty, UintParamRefinement, Value,
};
use crate::mesh::voxel_cloud::VoxelCloud;

#[derive(Debug, PartialEq)]
pub enum FuncVoxelTransformError {
    WeldFailed,
    TransformFailed,
    VoxelDimensionZero,
}

impl fmt::Display for FuncVoxelTransformError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FuncVoxelTransformError::WeldFailed => write!(
                f,
                "Welding of separate voxels failed due to high welding proximity tolerance"
            ),
            FuncVoxelTransformError::TransformFailed => {
                write!(f, "Voxel cloud transformation failed")
            }
            FuncVoxelTransformError::VoxelDimensionZero => {
                write!(f, "Voxel dimension is not larger than zero")
            }
        }
    }
}

impl error::Error for FuncVoxelTransformError {}

pub struct FuncVoxelTransform;

impl Func for FuncVoxelTransform {
    fn info(&self) -> &FuncInfo {
        &FuncInfo {
            name: "Voxel Transform",
            return_value_name: "VoxelTransformed Mesh",
        }
    }

    fn flags(&self) -> FuncFlags {
        FuncFlags::PURE
    }

    fn param_info(&self) -> &[ParamInfo] {
        &[
            ParamInfo {
                name: "Mesh",
                refinement: ParamRefinement::Mesh,
                optional: false,
            },
            ParamInfo {
                name: "Voxel Size",
                refinement: ParamRefinement::Float(FloatParamRefinement {
                    default_value: Some(1.0),
                    min_value: Some(f32::MIN_POSITIVE),
                    max_value: None,
                }),
                optional: false,
            },
            ParamInfo {
                name: "Grow",
                refinement: ParamRefinement::Uint(UintParamRefinement {
                    default_value: Some(0),
                    min_value: None,
                    max_value: None,
                }),
                optional: false,
            },
            ParamInfo {
                name: "Fill closed volumes",
                refinement: ParamRefinement::Boolean(BooleanParamRefinement {
                    default_value: true,
                }),
                optional: false,
            },
            ParamInfo {
                name: "Translate",
                refinement: ParamRefinement::Float3(Float3ParamRefinement {
                    default_value_x: Some(0.0),
                    min_value_x: None,
                    max_value_x: None,
                    default_value_y: Some(0.0),
                    min_value_y: None,
                    max_value_y: None,
                    default_value_z: Some(0.0),
                    min_value_z: None,
                    max_value_z: None,
                }),
                optional: false,
            },
            ParamInfo {
                name: "Rotate (deg)",
                refinement: ParamRefinement::Float3(Float3ParamRefinement {
                    default_value_x: Some(0.0),
                    min_value_x: None,
                    max_value_x: None,
                    default_value_y: Some(0.0),
                    min_value_y: None,
                    max_value_y: None,
                    default_value_z: Some(0.0),
                    min_value_z: None,
                    max_value_z: None,
                }),
                optional: false,
            },
            ParamInfo {
                name: "Scale",
                refinement: ParamRefinement::Float3(Float3ParamRefinement {
                    default_value_x: Some(1.0),
                    min_value_x: None,
                    max_value_x: None,
                    default_value_y: Some(1.0),
                    min_value_y: None,
                    max_value_y: None,
                    default_value_z: Some(1.0),
                    min_value_z: None,
                    max_value_z: None,
                }),
                optional: false,
            },
        ]
    }

    fn return_ty(&self) -> Ty {
        Ty::Mesh
    }

    fn call(
        &mut self,
        args: &[Value],
        _log: &mut dyn FnMut(LogMessage),
    ) -> Result<Value, FuncError> {
        let mesh = args[0].unwrap_mesh();
        let voxel_dimension = args[1].unwrap_float();

        if voxel_dimension <= 0.0 {
            return Err(FuncError::new(FuncVoxelTransformError::VoxelDimensionZero));
        }

        let growth_iterations = args[2].unwrap_uint();
        let fill = args[3].unwrap_boolean();
        let translate = Vector3::from(args[4].unwrap_float3());
        let rotate = args[5].unwrap_float3();
        let scale = args[6].unwrap_float3();

        let mut voxel_cloud = VoxelCloud::from_mesh(
            mesh,
            &Vector3::new(voxel_dimension, voxel_dimension, voxel_dimension),
        );
        for _ in 0..growth_iterations {
            voxel_cloud.grow_volume();
        }

        if fill {
            voxel_cloud.fill_volumes();
        }

        let rotation = (rotate[0], rotate[1], rotate[2]);

        let scaling = (scale[0], scale[1], scale[2]);

        if let Some(transformed_vc) = VoxelCloud::from_voxel_cloud_transformed(
            &voxel_cloud,
            voxel_dimension,
            &translate,
            &rotation,
            &scaling,
        ) {
            match transformed_vc.to_mesh() {
                Some(value) => Ok(Value::Mesh(Arc::new(value))),
                None => Err(FuncError::new(FuncVoxelTransformError::WeldFailed)),
            }
        } else {
            Err(FuncError::new(FuncVoxelTransformError::TransformFailed))
        }
    }
}