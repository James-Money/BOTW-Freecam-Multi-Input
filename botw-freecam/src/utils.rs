use winapi::um::winuser;
use std::ffi::CString;

const MINIMUM_ENGINE_SPEED: f32 = 1e-3;

/// Keys that aren't contained in the VirtualKeys from the Windows API.
#[repr(i32)]
pub enum Keys {
    A = 0x41,
    D = 0x44,
    E = 0x45,
    Q = 0x51,
    S = 0x53,
    W = 0x57,
}

#[derive(Default, Debug)]
pub struct Input {
    pub engine_speed: f32,
    // Deltas with X and Y
    pub delta_pos: (f32, f32),
    pub delta_focus: (f32, f32),
    pub delta_rotation: f32,

    pub delta_altitude: f32,

    pub change_active: bool,
    pub is_active: bool,

    pub fov: f32,

    // #[cfg(debug_assertions)]
    pub deattach: bool,
}

impl Input {
    pub fn new() -> Input {
        Self {
            fov: 0.92,
            engine_speed: MINIMUM_ENGINE_SPEED,
            ..Input::default()
        }
    }

    pub fn reset(&mut self) {
        self.delta_pos = (0., 0.);
        self.delta_focus = (0., 0.);
        self.delta_altitude = 0.;
        self.change_active = false;

        #[cfg(debug_assertions)]
        {
            self.deattach = false;
        }
    }

    pub fn sanitize(&mut self) {
        if self.fov < 1e-3 {
            self.fov = 0.01;
        }
        if self.fov > 3.12 {
            self.fov = 3.12;
        }

        if self.engine_speed < MINIMUM_ENGINE_SPEED {
            self.engine_speed = MINIMUM_ENGINE_SPEED;
        }
    }
}

pub fn handle_keyboard(input: &mut Input) {
    macro_rules! handle_state {
            ([ $key_pos:expr, $key_neg:expr, $var:ident, $val:expr ]; $($tt:tt)*) => {
                handle_state!([$key_pos, $key_neg, $var = $val, $var = - $val]; $($tt)*);
            };

            ([ $key_pos:expr, $key_neg:expr, $pos_do:expr, $neg_do:expr ]; $($tt:tt)*) => {
                if (winuser::GetAsyncKeyState($key_pos as i32) as u32 & 0x8000) != 0 {
                    $pos_do;
                }

                if (winuser::GetAsyncKeyState($key_neg as i32) as u32 & 0x8000) != 0 {
                    $neg_do;
                }
                handle_state!($($tt)*);
            };

            () => {}
        }

    unsafe {
        handle_state! {
            [Keys::W, Keys::S, input.delta_pos.1 = 0.2, input.delta_pos.1 = -0.2];
            [Keys::A, Keys::D, input.delta_pos.0 = 0.2, input.delta_pos.0 = -0.2];
            [Keys::Q, Keys::E, input.delta_altitude = 0.2, input.delta_altitude = -0.2];
            [winuser::VK_UP, winuser::VK_DOWN, input.delta_focus.1 = -0.05, input.delta_focus.1 = 0.05];
            [winuser::VK_LEFT, winuser::VK_RIGHT, input.delta_focus.0 = -0.05, input.delta_focus.0 = 0.05];
            [winuser::VK_F2, winuser::VK_F3, input.change_active = true, input.change_active = false];
        }
    }
}

pub fn error_message(message: &str) {
    let title = CString::new("Error while patching").unwrap();
    let message = CString::new(message).unwrap();

    unsafe {
        winapi::um::winuser::MessageBoxA(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            0x10,
        );
    }
}

#[cfg(allow_deadcode)]
pub fn handle_controller(input: &mut Input, func: fn(u32, &mut xinput::XINPUT_STATE) -> u32) {
    let mut xs: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };
    func(0, &mut xs);

    let gp = xs.Gamepad;
    println!("{:x}", gp.wButtons);

    // check camera activation
    if (gp.wButtons & (0x200 | 0x80)) == (0x200 | 0x80) {
        input.change_active = true;
    }

    #[cfg(debug_assertions)]
    if (gp.wButtons & (0x1000 | 0x4000)) == (0x1000 | 0x4000) {
        input.deattach = true;
    }

    // Update the camera changes only if it's listening
    if !input.is_active {
        return;
    }

    // modify speed
    if (gp.wButtons & 0x4) != 0 {
        input.engine_speed -= 0.01;
    }
    if (gp.wButtons & 0x8) != 0 {
        input.engine_speed += 0.01;
    }

    if (gp.wButtons & (0x200)) != 0 {
        input.delta_rotation += 0.01;
    }

    if (gp.wButtons & (0x100)) != 0 {
        input.delta_rotation -= 0.01;
    }

    if (gp.wButtons & (0x200 | 0x100)) == (0x200 | 0x100) {
        input.delta_rotation = 0.;
    }

    if gp.bLeftTrigger > 150 {
        input.fov -= 0.01;
    }

    if gp.bRightTrigger > 150 {
        input.fov += 0.01;
    }

    macro_rules! dead_zone {
        ($val:expr) => {
            if ($val < DEADZONE) && ($val > -DEADZONE) {
                0
            } else {
                $val
            }
        };
    }

    input.delta_pos.0 = -(dead_zone!(gp.sThumbLX) as f32) / ((i16::MAX as f32) * 1e2);
    input.delta_pos.1 = (dead_zone!(gp.sThumbLY) as f32) / ((i16::MAX as f32) * 1e2);

    input.delta_focus.0 = (dead_zone!(gp.sThumbRX) as f32) / ((i16::MAX as f32) * 1e2);
    input.delta_focus.1 = -(dead_zone!(gp.sThumbRY) as f32) / ((i16::MAX as f32) * 1e2);
}