use kepler_wgpu::{core::coord::Base, data::ct_volume::CTVolume, get_render_app};
use std::{thread, time::Duration};
fn main() {
    let mut app = pollster::block_on(get_render_app()).expect("synthetic native startup");
    let controls = app.get_glcanvas();
    thread::spawn(move || {
        let pause = || thread::sleep(Duration::from_secs(3));
        eprintln!("SMOKE: missing-volume mesh rejection");
        controls.set_render_mode(
            2, true, false, 0., 0., 0., 0., 0., 0., None, None, 200., 800., None, 0,
        );
        pause();
        let values = (0..8)
            .flat_map(|z| {
                (0..16)
                    .flat_map(move |y| (0..16).map(move |x| (-1024 + 128 * x + 8 * y + z) as i16))
            })
            .collect();
        let vol = CTVolume::new((16, 16, 8), (1., 1., 1.), values, Base::default());
        controls.load_data_from_ct_volume(&vol);
        pause();
        eprintln!("SMOKE: modified MPR settings and orientation");
        controls.set_scale(0, 1.3);
        controls.set_pan(0, 0.1, 0.2);
        controls.set_window_level(0, 0.);
        controls.set_slice_mm(0, 2.);
        controls.set_render_mode(
            0,
            true,
            false,
            0.,
            0.,
            0.,
            0.,
            0.,
            0.,
            None,
            Some(0),
            200.,
            800.,
            None,
            3,
        );
        pause();
        eprintln!("SMOKE: single MIP");
        controls.set_render_mode(
            1, true, false, 0., 0., 0., 0., 0., 0., None, None, 200., 800., None, 0,
        );
        pause();
        eprintln!("SMOKE: single mesh generation");
        controls.set_render_mode(
            2,
            false,
            false,
            0.,
            0.,
            0.,
            0.,
            0.,
            0.,
            Some(0),
            None,
            200.,
            800.,
            None,
            0,
        );
        pause();
        controls.set_mesh_scale(0, 1.3);
        controls.set_mesh_pan(0, 0.1, 0.2);
        eprintln!("SMOKE: mixed MIP/mesh, resize");
        controls.set_render_mode(
            3,
            true,
            false,
            0.,
            0.,
            0.,
            0.,
            0.,
            0.,
            Some(3),
            None,
            200.,
            800.,
            Some(2),
            0,
        );
        controls.resize(640, 480);
        pause();
        eprintln!("SMOKE: invalid orientation leaves old layout");
        controls.set_render_mode(
            0, true, false, 0., 0., 0., 0., 0., 0., None, None, 200., 800., None, 9,
        );
        pause();
        eprintln!("SMOKE: final MPR and quit");
        controls.set_render_mode(
            0, true, false, 0., 0., 0., 0., 0., 0., None, None, 200., 800., None, 0,
        );
        pause();
        controls.quit();
    });
    pollster::block_on(app.run());
    eprintln!("SMOKE: normal return");
}
