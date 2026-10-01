use anyhow::Result;
use enki::*;
use glam::Vec3;
use glfw::{Action, Context, Key, WindowEvent, WindowHint, WindowMode};
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

#[derive(Copy, Clone, Debug)]
pub struct RaytraceParams {
    pub time: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
}

#[derive(Copy, Clone, Debug)]
enum RayHit {
    Ground { pos: Vec3, normal: Vec3, dist: f32 },
    Metaball { normal: Vec3, dist: f32 },
    Sky,
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

fn scene_map(p: Vec3, time: f32) -> f32 {
    let ground = p.y + 0.8;

    let balls = [
        (Vec3::new(0.0, (time * 1.5).sin() * 0.3 + 0.3, 0.0), 0.75),
        (
            Vec3::new(
                (time * 1.2).cos() * 1.3,
                0.4 + (time * 1.8).sin() * 0.3,
                (time * 1.2).sin() * 1.3,
            ),
            0.45,
        ),
        (
            Vec3::new(
                (time * 0.9 + std::f32::consts::PI).cos() * 1.1,
                0.2,
                (time * 0.1 + std::f32::consts::PI).sin() * 1.1,
            ),
            0.35,
        ),
    ];

    let metaballs = balls
        .iter()
        .map(|&(center, radius)| (p - center).length() - radius)
        .fold(f32::MAX, |acc, d| smin(acc, d, 0.32));

    ground.min(metaballs)
}

fn calc_normal(p: Vec3, time: f32) -> Vec3 {
    let e = 0.002;

    let k1 = Vec3::new(1.0, -1.0, -1.0);
    let k2 = Vec3::new(-1.0, -1.0, 1.0);
    let k3 = Vec3::new(-1.0, 1.0, -1.0);
    let k4 = Vec3::new(1.0, 1.0, 1.0);

    let n = k1 * scene_map(p + k1 * e, time)
        + k2 * scene_map(p + k2 * e, time)
        + k3 * scene_map(p + k3 * e, time)
        + k4 * scene_map(p + k4 * e, time);

    n.normalize()
}

fn raymarch(ro: Vec3, rd: Vec3, time: f32) -> RayHit {
    let mut t = 0.0;
    for _ in 0..64 {
        let p = ro + rd * t;
        let d = scene_map(p, time);
        if d < 0.002 {
            let n = calc_normal(p, time);
            return if p.y < -0.78 {
                RayHit::Ground {
                    pos: p,
                    normal: n,
                    dist: t,
                }
            } else {
                RayHit::Metaball { normal: n, dist: t }
            };
        }
        t += d;
        if t > 25.0 {
            break;
        }
    }
    RayHit::Sky {}
}

#[nam]
pub fn raymarching_nam(space: &Space, pixel: &mut u32, params: RaytraceParams) {
    if !space.in_bounds_xy() {
        return;
    }

    let x = space.x as f32;
    let y = space.y as f32;
    let w = space.size_x as f32;
    let h = space.size_y as f32;

    let uv_x = (x - w * 0.5) / h;
    let uv_y = -(y - h * 0.5) / h;

    let cam_dist = 3.8;
    let cam_angle = params.time * 0.35 + (params.mouse_x - 0.5) * 3.0;
    let ro = Vec3::new(
        cam_angle.sin() * cam_dist,
        1.4 + (params.mouse_y - 0.5) * 2.0,
        cam_angle.cos() * cam_dist,
    );

    space.index();

    let target = Vec3::new(0.0, 0.3, 0.0);
    let cz = (target - ro).normalize();
    let cx = Vec3::Y.cross(cz).normalize();
    let cy = cz.cross(cx);
    let rd = (uv_x * cx + uv_y * cy + 1.4 * cz).normalize();

    let hit = raymarch(ro, rd, params.time);

    let (final_color, alpha) = match hit {
        RayHit::Ground { pos, normal, dist } => {
            let sun_dir = Vec3::new(3.0, 3.0, 0.8).normalize();
            let diff = normal.dot(sun_dir).max(0.0);
            let reflect_dir = rd - 2.0 * rd.dot(normal) * normal;
            let spec = reflect_dir.dot(sun_dir).max(0.0).powf(32.0);

            let grid_x = (pos.x * 2.0).floor() as i32;
            let grid_z = (pos.z * 2.0).floor() as i32;
            let base_color = if (grid_x + grid_z) % 2 == 0 {
                Vec3::new(0.15, 0.18, 0.25)
            } else {
                Vec3::new(0.25, 0.30, 0.45)
            };

            let ambient = Vec3::new(0.08, 0.09, 0.15);
            let light = ambient + base_color * diff * 1.2 + Vec3::new(1.0, 0.95, 0.85) * spec * 1.4;
            let fog = (dist / 50.0).clamp(0.0, 1.0);

            (light.lerp(Vec3::new(0.05, 0.08, 0.15), fog), 1.0)
        }

        RayHit::Metaball { normal, dist } => {
            let sun_dir = Vec3::new(3.0, 3.0, 0.8).normalize();
            let diff = normal.dot(sun_dir).max(0.0);
            let reflect_dir = rd - 2.0 * rd.dot(normal) * normal;
            let spec = reflect_dir.dot(sun_dir).max(0.0).powf(32.0);

            let base_color = Vec3::new(0.95, 0.25, 0.55).lerp(
                Vec3::new(0.20, 0.85, 0.95),
                (normal.y * 0.5 + 0.5).clamp(0.0, 1.0),
            );

            let ambient = Vec3::new(0.08, 0.09, 0.15);
            let light = ambient + base_color * diff * 1.2 + Vec3::new(1.0, 0.95, 0.85) * spec * 1.4;
            let fog = (dist / 50.0).clamp(0.0, 1.0);

            (light.lerp(Vec3::new(0.05, 0.08, 0.15), fog), 1.0)
        }

        RayHit::Sky { .. } => (Vec3::ZERO, 0.1),
    };

    *pixel = space.set_rgba_color(final_color.x, final_color.y, final_color.z, alpha);
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum ExecutionMode {
    Gpu,
    Cpu,
}

fn main() -> Result<()> {
    println!("ENKI SDF [GPU / CPU]");
    println!("Click space to toggle between GPU (Enki) and CPU (Rayon)");

    let mut glfw = glfw::init(glfw::fail_on_errors).expect("[GLFW] Failed to initialize");
    glfw.window_hint(WindowHint::Resizable(true));
    glfw.window_hint(WindowHint::TransparentFramebuffer(true));
    // glfw.window_hint(WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (mut window, events) = glfw
        .create_window(WIDTH, HEIGHT, "Enki 3D Raytracer", WindowMode::Windowed)
        .expect("[GLFW] Failed to create window");

    println!(
        "Is Framebuffer Transparent? {}",
        window.is_framebuffer_transparent()
    );

    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_framebuffer_size_polling(true);

    let window_handle = Arc::new(window);
    let mut current_width = WIDTH;
    let mut current_height = HEIGHT;

    let enki = Enki::init_windowed(window_handle.clone(), current_width, current_height);

    let total_pixels = (current_width * current_height) as usize;
    let mut gpu_pixels = gpu_vec![0u32; total_pixels];
    let mut cpu_pixels = vec![0u32; total_pixels];

    let mut mode = ExecutionMode::Gpu;
    let start_time = Instant::now();
    let mut last_frame_time = Instant::now();
    let mut fps_timer = Instant::now();
    let mut frame_count = 0u32;
    let mut fps;

    let mut mouse_x = 0.5f32;
    let mut mouse_y = 0.5f32;

    let mut is_running = true;

    while is_running && !window_handle.should_close() {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            match event {
                WindowEvent::Close => is_running = false,
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => is_running = false,
                WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                    mode = match mode {
                        ExecutionMode::Gpu => {
                            println!("[Mode Switch] ──> Switched to CPU (Rayon)");
                            ExecutionMode::Cpu
                        }
                        ExecutionMode::Cpu => {
                            println!("[Mode Switch] ──> Switched to GPU (Enki)");
                            ExecutionMode::Gpu
                        }
                    };
                }
                WindowEvent::CursorPos(x, y) => {
                    mouse_x = (x as f32 / current_width as f32).clamp(0.0, 1.0);
                    mouse_y = (y as f32 / current_height as f32).clamp(0.0, 1.0);
                }
                _ => {}
            }
        }

        let (fb_w, fb_h) = window_handle.get_framebuffer_size();
        if fb_w > 0 && fb_h > 0 && (fb_w as u32 != current_width || fb_h as u32 != current_height) {
            current_width = fb_w as u32;
            current_height = fb_h as u32;

            let _ = enki.resize(current_width, current_height);

            let total = (current_width * current_height) as usize;
            gpu_pixels = gpu_vec![0u32; total];
            cpu_pixels = vec![0u32; total];
        }

        let total_elapsed = start_time.elapsed().as_secs_f32();
        let dt = last_frame_time.elapsed().as_secs_f32();
        last_frame_time = Instant::now();

        frame_count += 1;
        if fps_timer.elapsed().as_secs_f32() >= 0.5 {
            fps = frame_count as f32 / fps_timer.elapsed().as_secs_f32();
            frame_count = 0;
            fps_timer = Instant::now();

            let mode_str = match mode {
                ExecutionMode::Gpu => "GPU: Enki Runtime -- intel UHD 620",
                ExecutionMode::Cpu => "CPU: Rayon Multi-threaded -- intel 7 gen 8",
            };
            let title = format!(
                "Enki SDF - [{}] | {:.1} FPS ({:.2} ms) | [{}x{}] (Press [Space] to toggle)",
                mode_str,
                fps,
                dt * 1000.0,
                current_width,
                current_height
            );

            if let Ok(c_title) = std::ffi::CString::new(title) {
                unsafe {
                    glfw::ffi::glfwSetWindowTitle(window_handle.window_ptr(), c_title.as_ptr());
                }
            }
        }

        let params = RaytraceParams {
            time: total_elapsed,
            mouse_x,
            mouse_y,
        };

        match mode {
            ExecutionMode::Gpu => {
                let space = Space::gpu_xy(current_width as usize, current_height as usize);
                enki.flow(|frame| {
                    raymarching_nam.run(&space, &mut gpu_pixels, GpuParam::new(params));
                    frame.present(&gpu_pixels);
                });
            }

            ExecutionMode::Cpu => {
                let width = current_width as usize;
                let height = current_height as usize;

                cpu_pixels
                    .par_chunks_mut(width)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for (x, pixel) in row.iter_mut().enumerate() {
                            let space_cpu = Space::cpu_xy(x, y, width, height);
                            raymarching_nam(&space_cpu, pixel, params);
                        }
                    });

                gpu_pixels.copy_from_slice(&cpu_pixels);
                enki.flow(|frame| {
                    frame.present(&gpu_pixels);
                });
            }
        }
    }

    println!("[Raytracer] Terminated cleanly.");
    Ok(())
}
