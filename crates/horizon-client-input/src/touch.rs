//! Multi-touch routing in GUI coordinates. Each finger owns its initial role.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sneak,
    Attack,
    Use,
    Inventory,
    Pause,
    Scheme,
}
#[derive(Clone, Copy, Debug)]
pub struct Rect(pub f32, pub f32, pub f32, pub f32);
impl Rect {
    pub fn contains(self, p: [f32; 2]) -> bool {
        p[0] >= self.0 && p[0] < self.0 + self.2 && p[1] >= self.1 && p[1] < self.1 + self.3
    }
}
/// Keep both drawing and hit testing on this same layout. Margins leave room for a notch.
pub fn buttons([w, h]: [f32; 2]) -> Vec<(Button, Rect)> {
    use Button::*;
    let s = 24.0;
    vec![
        (Forward, Rect(46.0, h - 116.0, s, s)),
        (Back, Rect(46.0, h - 64.0, s, s)),
        (Left, Rect(20.0, h - 90.0, s, s)),
        (Right, Rect(72.0, h - 90.0, s, s)),
        (Jump, Rect(w - 46.0, h - 90.0, s, s)),
        (Sneak, Rect(w - 72.0, h - 64.0, s, s)),
        (Attack, Rect(w - 98.0, h - 116.0, s, s)),
        (Use, Rect(w - 46.0, h - 142.0, s, s)),
        (Inventory, Rect(w / 2.0 + 94.0, h - 26.0, s, s)),
        (Pause, Rect(w - 46.0, 18.0, s, s)),
        (Scheme, Rect(20.0, 18.0, s, s)),
    ]
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Hold(Button, bool),
    Look([f32; 2]),
    Slot(usize),
    Tap(Button),
}
#[derive(Clone, Copy, Debug)]
enum Role {
    Button(Button),
    Move([f32; 2], [f32; 2]),
    Look([f32; 2]),
    Ignore,
}
#[derive(Clone, Copy)]
pub enum Phase {
    Start,
    Move,
    End,
}
#[derive(Default)]
pub struct TouchControls {
    fingers: BTreeMap<u64, Role>,
    held: Vec<Button>,
    pub dpad: bool,
}
impl TouchControls {
    pub fn held(&self, b: Button) -> bool {
        self.held.contains(&b)
    }
    pub fn reset(&mut self) -> Vec<Action> {
        self.fingers.clear();
        self.held
            .drain(..)
            .map(|b| Action::Hold(b, false))
            .collect()
    }
    pub fn event(&mut self, id: u64, phase: Phase, p: [f32; 2], size: [f32; 2]) -> Vec<Action> {
        use Button::*;
        let mut out = Vec::new();
        match phase {
            Phase::Start => {
                let hit = buttons(size)
                    .into_iter()
                    .find(|(_, r)| r.contains(p))
                    .map(|(b, _)| b);
                let role = if let Some(b) = hit {
                    if matches!(b, Inventory | Pause | Scheme) {
                        if b == Scheme {
                            out.extend(self.reset());
                            self.dpad = !self.dpad;
                        } else {
                            out.push(Action::Tap(b));
                        }
                        Role::Ignore
                    } else if !self.dpad && matches!(b, Forward | Back | Left | Right) {
                        // Fixed joystick centre, drawn at the middle of the movement pad.
                        Role::Move([58.0, size[1] - 78.0], p)
                    } else {
                        Role::Button(b)
                    }
                } else if p[1] >= size[1] - 22.0
                    && p[0] >= size[0] / 2.0 - 90.0
                    && p[0] < size[0] / 2.0 + 90.0
                {
                    out.push(Action::Slot(
                        ((p[0] - (size[0] / 2.0 - 90.0)) / 20.0) as usize,
                    ));
                    Role::Ignore
                } else if !self.dpad
                    && p[0] < 110.0
                    && p[1] > size[1] - 145.0
                    && p[1] < size[1] - 28.0
                {
                    Role::Move([58.0, size[1] - 78.0], p)
                } else {
                    Role::Look(p)
                };
                self.fingers.insert(id, role);
            }
            Phase::Move => {
                if let Some(role) = self.fingers.get_mut(&id) {
                    match role {
                        Role::Move(_, position) => {
                            *position = p;
                        }
                        Role::Look(previous) => {
                            out.push(Action::Look([p[0] - previous[0], p[1] - previous[1]]));
                            *previous = p;
                        }
                        Role::Button(b)
                            if self.dpad && matches!(b, Forward | Back | Left | Right) =>
                        {
                            if let Some((next, _)) = buttons(size).into_iter().find(|(b, r)| {
                                matches!(b, Forward | Back | Left | Right) && r.contains(p)
                            }) {
                                *b = next;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Phase::End => {
                self.fingers.remove(&id);
            }
        }
        let mut next = Vec::new();
        for role in self.fingers.values() {
            let mut add = |b| {
                if !next.contains(&b) {
                    next.push(b);
                }
            };
            match *role {
                Role::Button(b) => add(b),
                Role::Move(origin, position) => {
                    let [x, y] = [position[0] - origin[0], position[1] - origin[1]];
                    for (active, b) in [
                        (x < -8.0, Left),
                        (x > 8.0, Right),
                        (y < -8.0, Forward),
                        (y > 8.0, Back),
                    ] {
                        if active {
                            add(b);
                        }
                    }
                }
                _ => {}
            }
        }
        for &b in &self.held {
            if !next.contains(&b) {
                out.push(Action::Hold(b, false));
            }
        }
        for &b in &next {
            if !self.held.contains(&b) {
                out.push(Action::Hold(b, true));
            }
        }
        self.held = next;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SIZE: [f32; 2] = [400.0, 240.0];
    #[test]
    fn movement_survives_other_fingers_and_releases_on_cancel() {
        let mut t = TouchControls::default();
        assert!(
            t.event(1, Phase::Start, [58.0, 130.0], SIZE)
                .contains(&Action::Hold(Button::Forward, true))
        );
        t.event(2, Phase::Start, [200.0, 100.0], SIZE);
        assert_eq!(
            t.event(2, Phase::Move, [210.0, 104.0], SIZE),
            vec![Action::Look([10.0, 4.0])]
        );
        t.event(3, Phase::Start, [360.0, 155.0], SIZE);
        assert!(t.held(Button::Jump) && t.held(Button::Forward));
        t.event(3, Phase::End, [0.0, 0.0], SIZE);
        assert!(t.held(Button::Forward));
        assert_eq!(t.reset(), vec![Action::Hold(Button::Forward, false)]);
        assert!(t.event(2, Phase::Move, [240.0, 104.0], SIZE).is_empty());
    }
    #[test]
    fn hotbar_edges_and_finger_ownership() {
        let mut t = TouchControls::default();
        assert_eq!(
            t.event(1, Phase::Start, [110.0, 230.0], SIZE),
            vec![Action::Slot(0)]
        );
        assert_eq!(
            t.event(2, Phase::Start, [289.0, 230.0], SIZE),
            vec![Action::Slot(8)]
        );
        assert!(t.event(1, Phase::Move, [200.0, 100.0], SIZE).is_empty());
    }
    #[test]
    fn two_fingers_on_same_button_do_not_release_each_other() {
        let mut t = TouchControls::default();
        t.event(1, Phase::Start, [360.0, 155.0], SIZE);
        t.event(2, Phase::Start, [360.0, 155.0], SIZE);
        assert!(t.event(1, Phase::End, [0.0, 0.0], SIZE).is_empty());
        assert_eq!(
            t.event(2, Phase::End, [0.0, 0.0], SIZE),
            vec![Action::Hold(Button::Jump, false)]
        );
    }
    #[test]
    fn joystick_can_change_direction_while_look_finger_moves() {
        let mut t = TouchControls::default();
        t.event(1, Phase::Start, [58.0, 130.0], SIZE);
        t.event(2, Phase::Start, [200.0, 100.0], SIZE);
        let actions = t.event(1, Phase::Move, [90.0, 190.0], SIZE);
        assert!(actions.contains(&Action::Hold(Button::Forward, false)));
        assert!(t.held(Button::Right) && t.held(Button::Back));
        t.event(2, Phase::Move, [210.0, 100.0], SIZE);
        assert!(t.held(Button::Right) && t.held(Button::Back));
        let actions = t.event(1, Phase::End, [90.0, 190.0], SIZE);
        assert!(actions.contains(&Action::Hold(Button::Right, false)));
        assert!(actions.contains(&Action::Hold(Button::Back, false)));
    }
    #[test]
    fn changing_scheme_releases_old_fingers_and_dpad_can_slide() {
        let mut t = TouchControls::default();
        t.event(1, Phase::Start, [58.0, 130.0], SIZE);
        assert!(
            t.event(2, Phase::Start, [25.0, 20.0], SIZE)
                .contains(&Action::Hold(Button::Forward, false))
        );
        assert!(t.dpad);
        assert!(t.event(1, Phase::Move, [58.0, 130.0], SIZE).is_empty());
        t.event(3, Phase::Start, [58.0, 130.0], SIZE);
        t.event(3, Phase::Move, [80.0, 160.0], SIZE);
        assert!(!t.held(Button::Forward) && t.held(Button::Right));
    }
}
