extern crate nalgebra_glm as glm;
use std::{ mem, ptr, os::raw::c_void };
use std::thread;
use std::sync::{Mutex, Arc, RwLock};

mod shader;
mod util;
mod mesh;
mod scene_graph;

use glutin::event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState::{Pressed, Released}, VirtualKeyCode::{self, *}};
use glutin::event_loop::ControlFlow;
use glm::{Vec3, Vec2};
use crate::shader::Shader;
use std::f32::consts::PI;

const SCREEN_W: u32 = 800;
const SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //
// The names should be pretty self explanatory
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()



// == // Modify and complete the function below for the first task
unsafe fn create_mesh_vao(mesh: &mesh::Mesh) -> u32 {
    create_vao(&mesh.vertices, &mesh.normals, &mesh.colors, &mesh.indices)
}

unsafe fn create_vao(vertices: &Vec<f32>, normals: &Vec<f32>, colours: &Vec<f32>, indices: &Vec<u32>) -> u32 {
    let mut vao_index: u32 = 0;
    gl::GenVertexArrays(1, &mut vao_index);
    gl::BindVertexArray(vao_index);

    let mut buffer_index: u32 = 0;
    gl::GenBuffers(1, &mut buffer_index);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer_index);
    gl::BufferData(gl::ARRAY_BUFFER, byte_size_of_array(&vertices), pointer_to_array(&vertices), gl::STATIC_DRAW);
    gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(0);

    gl::GenBuffers(1, &mut buffer_index);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer_index);
    gl::BufferData(gl::ARRAY_BUFFER, byte_size_of_array(&normals), pointer_to_array(&normals), gl::STATIC_DRAW);
    gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(1);

    gl::GenBuffers(1, &mut buffer_index);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer_index);
    gl::BufferData(gl::ARRAY_BUFFER, byte_size_of_array(&colours), pointer_to_array(&colours), gl::STATIC_DRAW);
    gl::VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(2);

    gl::GenBuffers(1, &mut buffer_index);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer_index);
    gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, byte_size_of_array(&indices), pointer_to_array(&indices), gl::STATIC_DRAW);

    vao_index
}

unsafe fn draw_scene(root: &scene_graph::SceneNode, view_projection_matrix: &glm::Mat4) {
    // Check if node is drawable, set uniforms, draw
    if root.index_count != -1 {
        gl::UniformMatrix4fv(3, 1, gl::FALSE, view_projection_matrix.as_ptr());
        gl::BindVertexArray(root.vao_id);
        gl::DrawElements(gl::TRIANGLES, (3 * root.index_count), gl::UNSIGNED_INT, ptr::null());
    }
    // Recurse
    for &child in &root.children {
        draw_scene(&*child, view_projection_matrix);
    }
}

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(false)
        .with_inner_size(glutin::dpi::LogicalSize::new(SCREEN_W, SCREEN_H));
    let cb = glutin::ContextBuilder::new()
        .with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);
    
    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers. This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!("{}: {}", util::get_gl_string(gl::VENDOR), util::get_gl_string(gl::RENDERER));
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!("GLSL\t: {}", util::get_gl_string(gl::SHADING_LANGUAGE_VERSION));
        }

        // == // Set up your VAO here
        let terrain = mesh::Terrain::load("resources/lunarsurface.obj");
        let helicopter = mesh::Helicopter::load("resources/helicopter.obj");
        let mut vao_indices = Vec::<u32>::new();
        let mut index_counts = Vec::<i32>::new();
        unsafe {
            vao_indices.push(create_mesh_vao(&terrain));
            index_counts.push(terrain.index_count);
            for i in 0..4 {
                vao_indices.push(create_mesh_vao(&helicopter[i]));
                index_counts.push(helicopter[i].index_count);
            }
        }

        // Set up scene graph
        let mut root_node = scene_graph::SceneNode::new();
        let mut terrain_node = scene_graph::SceneNode::from_vao(vao_indices[0], index_counts[0]);
        let mut body_node = scene_graph::SceneNode::from_vao(vao_indices[1], index_counts[1]);
        let mut main_rotor_node = scene_graph::SceneNode::from_vao(vao_indices[2], index_counts[2]);
        let mut tail_rotor_node = scene_graph::SceneNode::from_vao(vao_indices[3], index_counts[3]);
        let mut door_node = scene_graph::SceneNode::from_vao(vao_indices[4], index_counts[4]);

        root_node.add_child(&terrain_node);
        terrain_node.add_child(&body_node);
        body_node.add_child(&main_rotor_node);
        body_node.add_child(&tail_rotor_node);
        body_node.add_child(&door_node);

        // Basic usage of shader helper
        // The code below returns a shader object, which contains the field .program_id
        // The snippet is not enough to do the assignment, and will need to be modified (outside of just using the correct path), but it only needs to be called once
        // shader::ShaderBuilder::new().attach_file("./path/to/shader").link();
        let mut simple_shader: Shader;
        unsafe {
            simple_shader = shader::ShaderBuilder::new().attach_file("./shaders/simple.vert")
                .attach_file("./shaders/simple.frag").link();
            gl::UseProgram(simple_shader.program_id);
        }

        // Used to demonstrate keyboard handling -- feel free to remove
        let mut pos: glm::Vec3 = glm::vec3(0.0, 0.0, 0.0);
        let mut ang: glm::Vec2 = glm::vec2(0.0, 0.0);

        fn mov(dir: &glm::Vec4, ang: &glm::Vec2) -> glm::Vec3 {
            let mut movement: glm::Mat4 = glm::identity();

            movement *= glm::rotation(-ang.x, &glm::vec3(0.0, 1.0, 0.0));
            movement *= glm::rotation(-ang.y, &glm::vec3(1.0, 0.0, 0.0));

            glm::vec4_to_vec3(&(movement * dir))
        }

        let first_frame_time = std::time::Instant::now();
        let mut last_frame_time = first_frame_time;
        // The main rendering loop
        loop {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(last_frame_time).as_secs_f32();
            last_frame_time = now;

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        VirtualKeyCode::A => {
                            pos += mov(&glm::vec4(100.0 * delta_time, 0.0, 0.0, 1.0), &ang);
                        },
                        VirtualKeyCode::D => {
                            pos += mov(&glm::vec4(100.0 * -delta_time, 0.0, 0.0, 1.0), &ang);
                        },
                        VirtualKeyCode::S => {
                            pos += mov(&glm::vec4(0.0, 0.0, 100.0 * -delta_time, 1.0), &ang);
                        },
                        VirtualKeyCode::W => {
                            pos += mov(&glm::vec4(0.0, 0.0, 100.0 * delta_time, 1.0), &ang);
                        },
                        VirtualKeyCode::LShift => {
                            pos += mov(&glm::vec4(0.0, 100.0 * delta_time, 0.0, 1.0), &ang);
                        },
                        VirtualKeyCode::Space => {
                            pos += mov(&glm::vec4(0.0, 100.0 * -delta_time, 0.0, 1.0), &ang);
                        },
                        VirtualKeyCode::Q => {
                            ang.x -= delta_time;

                        },
                        VirtualKeyCode::E => {
                            ang.x += delta_time;

                        },
                        VirtualKeyCode::R => {
                            ang.y -= delta_time;

                        },
                        VirtualKeyCode::F => {
                            ang.y += delta_time;
                        },

                        _ => { }
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {



                *delta = (0.0, 0.0);
            }

            unsafe {
                gl::ClearColor(0.163, 0.163, 0.163, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                let mut transform: glm::Mat4 = glm::identity();

                transform *= glm::perspective(1.0, PI / 2.0, 1.0, 1000.0);
                transform *= glm::translation(&glm::vec3(0.0, 0.0, -1.2));
                transform *= glm::rotation(ang.y, &glm::vec3(1.0, 0.0, 0.0));
                transform *= glm::rotation(ang.x, &glm::vec3(0.0, 1.0, 0.0));
                transform *= glm::translation(&pos);

                draw_scene(&root_node, &transform);
            }

            context.swap_buffers().unwrap();
        }
    });

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events get handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            },
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent { event: WindowEvent::KeyboardInput {
                input: KeyboardInput { state: key_state, virtual_keycode: Some(keycode), .. }, .. }, .. } => {

                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        },
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle escape separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    },
                    _ => { }
                }
            },
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            },
            _ => { }
        }
    });
}
