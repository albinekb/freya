use dioxus::prelude::*;
use freya_core::{accessibility::AccessibilityFocusDirection, prelude::EventMessage};
use freya_elements::{elements as dioxus_elements, events::KeyboardEvent};
use freya_hooks::{use_init_native_platform, use_platform};

#[allow(non_snake_case)]
#[component]
pub fn NativeContainer(children: Element) -> Element {
    let mut native_platform = use_init_native_platform();
    let platform = use_platform();

    let onkeydown = move |e: KeyboardEvent| {
        let allowed_to_navigate = native_platform.navigation_mark.peek().allowed();
        if e.key == Key::Tab && allowed_to_navigate {
            if e.modifiers.contains(Modifiers::SHIFT) {
                platform
                    .send(EventMessage::FocusPrevAccessibilityNode)
                    .unwrap();
            } else {
                platform
                    .send(EventMessage::FocusNextAccessibilityNode)
                    .unwrap();
            }
        } else if allowed_to_navigate
            && (e.modifiers.contains(Modifiers::SHIFT) && e.modifiers.contains(Modifiers::META))
            && (e.key == Key::ArrowLeft
                || e.key == Key::ArrowRight
                || e.key == Key::ArrowUp
                || e.key == Key::ArrowDown)
        {
            let dir = match e.key {
                Key::ArrowLeft => AccessibilityFocusDirection::Left,
                Key::ArrowRight => AccessibilityFocusDirection::Right,
                Key::ArrowUp => AccessibilityFocusDirection::Up,
                Key::ArrowDown => AccessibilityFocusDirection::Down,
                _ => unreachable!(),
            };
            println!("Focus with direction: {:?}", dir);
            platform
                .send(EventMessage::FocusAccessibilityNodeWithDirection(dir))
                .unwrap();
        } else {
            native_platform.navigation_mark.write().set_allowed(true)
        }
    };

    rsx!(rect {
        width: "100%",
        height: "100%",
        onkeydown,
        {children}
    })
}
