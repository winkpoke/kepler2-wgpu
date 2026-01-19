use glam::Mat4;
use kepler_wgpu::core::coord::Base;
use kepler_wgpu::data::CTVolume;
use kepler_wgpu::rendering::Orientation;

#[test]
fn debug_matrix_layout() {
    let volume = CTVolume {
        dimensions: (512, 512, 100),
        voxel_spacing: (1.0, 1.0, 2.0),
        voxel_data: vec![0i16; 512 * 512 * 100],
        base: Base {
            label: "test_volume".to_string(),
            matrix: Mat4::IDENTITY,
        },
    };

    let base = Orientation::Transverse.build_base(&volume);
    let m = base.matrix;

    println!("Matrix from build_transverse_base:");
    println!();
    println!("Column-major representation (glam default):");
    println!(
        "col(0) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(0)[0],
        m.col(0)[1],
        m.col(0)[2],
        m.col(0)[3]
    );
    println!(
        "col(1) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(1)[0],
        m.col(1)[1],
        m.col(1)[2],
        m.col(1)[3]
    );
    println!(
        "col(2) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(2)[0],
        m.col(2)[1],
        m.col(2)[2],
        m.col(2)[3]
    );
    println!(
        "col(3) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(3)[0],
        m.col(3)[1],
        m.col(3)[2],
        m.col(3)[3]
    );
    println!();

    println!("Row-major representation (how tests access it):");
    println!(
        "row(0) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(0)[0],
        m.col(1)[0],
        m.col(2)[0],
        m.col(3)[0]
    );
    println!(
        "row(1) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(0)[1],
        m.col(1)[1],
        m.col(2)[1],
        m.col(3)[1]
    );
    println!(
        "row(2) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(0)[2],
        m.col(1)[2],
        m.col(2)[2],
        m.col(3)[2]
    );
    println!(
        "row(3) = [{:>8.2}, {:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(0)[3],
        m.col(1)[3],
        m.col(2)[3],
        m.col(3)[3]
    );
    println!();

    println!("Column vectors (upper 3x3):");
    println!(
        "col0.xyz = [{:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(0).x,
        m.col(0).y,
        m.col(0).z
    );
    println!(
        "col1.xyz = [{:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(1).x,
        m.col(1).y,
        m.col(1).z
    );
    println!(
        "col2.xyz = [{:>8.2}, {:>8.2}, {:>8.2}]",
        m.col(2).x,
        m.col(2).y,
        m.col(2).z
    );
    println!();

    println!("Row vectors (upper 3x3, as accessed by tests):");
    println!(
        "row0 = [{:>8.2}, {:>8.2}, {:>8.2}] (col0.x, col1.x, col2.x)",
        m.col(0)[0],
        m.col(1)[0],
        m.col(2)[0]
    );
    println!(
        "row1 = [{:>8.2}, {:>8.2}, {:>8.2}] (col0.y, col1.y, col2.y)",
        m.col(0)[1],
        m.col(1)[1],
        m.col(2)[1]
    );
    println!(
        "row2 = [{:>8.2}, {:>8.2}, {:>8.2}] (col0.z, col1.z, col2.z)",
        m.col(0)[2],
        m.col(1)[2],
        m.col(2)[2]
    );
    println!();

    // Test what tests are checking
    println!("What tests check:");
    let row_0_len = (m.col(0)[0].powi(2) + m.col(1)[0].powi(2) + m.col(2)[0].powi(2)).sqrt();
    println!("Row 0 length: {}", row_0_len);

    let col_0_len = m.col(0).truncate().length();
    println!("Col 0 length: {}", col_0_len);

    // Determinant
    let mat3 = glam::Mat3::from_mat4(m);
    let det = mat3.determinant();
    println!("Mat3 determinant: {}", det);
}
