   //let patient_position = series.patient_position.clone().unwrap_or_else(|| String::from("HFS"));
        //                 let direction_matrix = match patient_position.as_ref() {
        //     "HFS" => Matrix4x4::from_array([
        //         column_direction.0, row_direction.0, slice_direction.0, 0.0,
        //         column_direction.1, row_direction.1, slice_direction.1, 0.0,
        //         column_direction.2, row_direction.2, slice_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     "FFS" => Matrix4x4::from_array([
        //         row_direction.0, column_direction.0, -slice_direction.0, 0.0,
        //         row_direction.1, column_direction.1, -slice_direction.1, 0.0,
        //         row_direction.2, column_direction.2, -slice_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     "HFP" => Matrix4x4::from_array([
        //         -row_direction.0, -column_direction.0, slice_direction.0, 0.0,
        //         -row_direction.1, -column_direction.1, slice_direction.1, 0.0,
        //         -row_direction.2, -column_direction.2, slice_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     "FFP" => Matrix4x4::from_array([
        //         -row_direction.0, -column_direction.0, -slice_direction.0, 0.0,
        //         -row_direction.1, -column_direction.1, -slice_direction.1, 0.0,
        //         -row_direction.2, -column_direction.2, -slice_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     "LFS" => Matrix4x4::from_array([
        //         slice_direction.0, column_direction.0, - row_direction.0, 0.0,
        //         slice_direction.1, column_direction.1, - row_direction.1, 0.0,
        //         slice_direction.2, column_direction.2, - row_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     "LFP" => Matrix4x4::from_array([
        //         -slice_direction.0, -column_direction.0, row_direction.0, 0.0,
        //         -slice_direction.1, -column_direction.1, row_direction.1, 0.0,
        //         -slice_direction.2, -column_direction.2, row_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     _ => Matrix4x4::from_array([
        //         row_direction.0, column_direction.0, slice_direction.0, 0.0,
        //         row_direction.1, column_direction.1, slice_direction.1, 0.0,
        //         row_direction.2, column_direction.2, slice_direction.2, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        // };
        
// CTVolume
        // let direction_matrix = match self.element_type.as_str() {
        //     "MET_SHORT" | "MET_INT16" => Matrix4x4::from_array([
        //         self.transform[0], self.transform[1], self.transform[2], 0.0,
        //         self.transform[3], self.transform[4], self.transform[5], 0.0,
        //         self.transform[6], self.transform[7], self.transform[8], 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     "MET_FLOAT" =>Matrix4x4::from_array([
        //         self.transform[2], self.transform[0], self.transform[1], 0.0,
        //         self.transform[5], self.transform[3], self.transform[4], 0.0,
        //         self.transform[8], self.transform[6], - self.transform[7], 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     ]),
        //     other => return Err(format!("Unsupported ElementType: {}", other)),
        // };

// EXPORT_DICOM
//     match header.element_type.as_str() {
//             "MET_SHORT" | "MET_INT16" => {
//         //     "MET_FLOAT" => {
        //         let transform: &[f32] = &header.transform;
        //         let (row_dir, slice_dir,col_dir) =orientation_dirs(transform);
        //         (col_dir, row_dir, slice_dir)

                // let col = self.dim[2]; // x:512
                // let row = self.dim[0]; // y:512
                // let depth = self.dim[1]; // z:300
                
                // let data = &self.data;
                // let mut voxel_data = data.clone();
                // for value in &mut voxel_data {
                //     if *value < -1024 {
                //         *value = -1024;
                //     }
                // }
                // let mut rotated_i16 = vec![0i16; row * col * depth];
                // for x in 0..col {
                //     for new_y_idx in 0..row {
                //         for new_z_idx in 0..depth {
                //             let old_x = x;
                //             let old_y = depth - 1 - new_z_idx;
                //             let old_z = new_y_idx;
                //             let old_idx = old_z * row * depth + old_y * row + old_x;
                //             let new_idx = new_z_idx * col * row + new_y_idx * col + x;
                //             rotated_i16[new_idx] = voxel_data[old_idx];
                //         }
                //     }
                // }

                // // series
                // let uid = "1.3.6.1.4.1.14519.5.2.1.6919.4624.2819497684894126";

                // // scaling matrix
                // let scaling_matrix = Matrix4x4::from_array([
                //     self.spacing[2], 0.0, 0.0, 0.0,
                //     0.0, self.spacing[0], 0.0, 0.0,
                //     0.0, 0.0, self.spacing[1], 0.0,
                //     0.0, 0.0, 0.0, 1.0,
                // ]);

                // // direction matrix
                // let direction_matrix = Matrix4x4::from_array([
                //     1.0, 0.0, 0.0, 0.0,
                //     0.0, 1.0, 0.0, 0.0,
                //     0.0, 0.0, 1.0, 0.0,
                //     0.0, 0.0, 0.0, 1.0,
                // ]);

                // // translation matrix
                // let translation_matrix = Matrix4x4::from_array([
                //     1.0, 0.0, 0.0, -127.0,
                //     0.0, 1.0, 0.0, -127.0,
                //     0.0, 0.0, 1.0, 127.0,
                //     0.0, 0.0, 0.0, 1.0,
                // ]);

                // // Multiply the scaling, direction, and translation matrices
                // let base_matrix = direction_matrix
                //     .multiply(&translation_matrix)
                //     .multiply(&scaling_matrix);

                // // Return the constructed CTVolume
                // CTVolume {
                //     dimensions: (col, row, depth),
                //     voxel_spacing: (self.spacing[2], self.spacing[0], self.spacing[1]),
                //     voxel_data: rotated_i16,
                //     base: Base {
                //         label: uid.to_string(),
                //         matrix: base_matrix,
                //     }
                // }