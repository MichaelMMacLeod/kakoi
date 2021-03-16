use crate::state::State;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};

pub fn create_window() {
    env_logger::init();
    let event_loop = EventLoop::new();

    let min_size: winit::dpi::PhysicalSize<u32> = (200, 200).into();
    let start_size: winit::dpi::PhysicalSize<u32> = (1920, 1080).into();
    let window = winit::window::WindowBuilder::new()
        .with_title("kakoi")
        .with_min_inner_size(min_size)
        .with_inner_size(start_size)
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let mut state = futures::executor::block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                }
                // get keyboard input, etc. here
                _ => {}
            },
            Event::RedrawRequested(_) => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SwapChainError::Lost) => state.recreate_swap_chain(),
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            _ => {}
        }
    })
}
