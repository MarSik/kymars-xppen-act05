use evdev::Key;

use crate::kbd_events::KeyStateChange;
use crate::layout::layer::Layer;
use crate::layout::types::KeyCoords;
use crate::layout::switcher::LayerSwitcher;
use crate::layout::types::KeymapEvent::{Kg, No, Lhold, Inh, Ltap, Lactivate, Pass, LhtK, LhtL, Klong, Khl, Khtl, Ldeactivate};
use crate::layout::keys::{G, S};

use self::testtime::TestTime;

#[non_exhaustive]
struct TestDevice;

impl TestDevice {
    pub(crate) const B01: KeyCoords = KeyCoords(0, 0, 0);
    pub(crate) const B02: KeyCoords = KeyCoords(0, 0, 1);
    pub(crate) const B03: KeyCoords = KeyCoords(0, 1, 0);
    pub(crate) const B04: KeyCoords = KeyCoords(0, 1, 1);
}

const DEFAULT_LAYER_CONFIG: Layer = Layer{
    status_on_reset: crate::layout::types::LayerStatus::LayerActive,
    inherit: None,
    on_active_keys: vec![],
    disable_active_on_press: false,
    on_timeout_layer: None,
    timeout: None,
    keymap: vec![],
    default_action: crate::layout::types::KeymapEvent::Pass,
};

#[track_caller]
fn assert_emitted_keys(layout: &mut LayerSwitcher, keys: Vec<(Key, bool)>) {
    let mut received = Vec::new();

    // Compute all registered keys. This is done every time instead of once,
    // but it makes the code simpler to write
    let registered_keys = layout.get_used_keys();

    // The test could be done directly in the closure, but the asserts then
    // report a wrong caller line, because track_caller is still unstable
    // for closures.
    layout.render(|k, v| {
        received.push((k, v));
    });

    let mut idx = 0;
    for (k, v) in received {
        assert!(idx < keys.len(), "Unexpected key {:?}/{}", k, v);
        assert_eq!(keys[idx].0, k, "Expected key {:?}/{} got {:?}/{}", keys[idx].0, keys[idx].1, k, v);
        assert_eq!(keys[idx].1, v, "Expected key {:?} state to be {} got {}", k, keys[idx].1, v);
        assert!(registered_keys.contains(&k), "Emitted key {:?} is not registered to the OS", k);
        idx += 1;
    }

    assert_eq!(idx, keys.len(), "Expected {} key presses. Got only {}.", keys.len(), idx);
}

// Single layer, basic key press and release test
fn basic_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ G().k(Key::KEY_LEFTALT).p(),   G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer];

    layers
}

mod testtime;

#[test]
fn test_basic_layout() {
    let layout_vec = basic_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();

    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTALT, true)]);

    // Test that long press will not break the key flow
    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(500));
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTALT, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t.advance_ms(10));
    assert_emitted_keys(&mut layout, vec![]);
}

// Dual layout, basic test simulating Shift behavior (hold to stay in the second layer)
// It also tests pass-through to lower layer and inheritance from inactive layer
fn basic_layered_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Lhold(1),              G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ G().k(Key::KEY_0).p(), Pass,          ],
            vec![ Inh          , G().k(Key::KEY_E).p(), ],
        ],
    ];

    let keymap_inh = vec![ // blocks
        vec![ // rows
            vec![ G().k(Key::KEY_1).p(), G().k(Key::KEY_9).p(), ],
            vec![ G().k(Key::KEY_2).p(), G().k(Key::KEY_3).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        inherit: Some(2),
        on_active_keys: vec![Key::KEY_LEFTSHIFT],
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let inh_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerDisabled,
        keymap: keymap_inh,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer, inh_layer];

    layers
}

#[test]
fn test_basic_layered_layout() {
    let layout_vec = basic_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();

    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    // Test that long press will not break the layer switch flow
    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(500));
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_E, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B03), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_2, true), (Key::KEY_2, false)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_basic_layered_layout_cross_release() {
    let layout_vec = basic_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true),]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

// Dual layout, basic test simulating dead-key (sticky) behavior (stay in the second layer until next key is pressed)
fn tap_layered_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Ltap(1),               G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ No,                    Inh,           ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        on_active_keys: vec![Key::KEY_LEFTSHIFT],
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}

#[test]
fn test_tap_layered_layout() {
    let layout_vec = tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_LEFTSHIFT, false), (Key::KEY_B, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_tap_layered_hold() {
    let layout_vec = tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false) ]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_LEFTSHIFT, false), (Key::KEY_E, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_tap_layered_hold_crossed() {
    let layout_vec = tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true) ]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, false) ]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_LEFTSHIFT, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, false)]);
}

#[test]
fn test_tap_layered_hold_dual_crossed() {
    let layout_vec = tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true) ]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_LEFTSHIFT, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, false) ]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, false)]);
}

#[test]
fn test_tap_layered_hold_dual_crossed_lifo() {
    let layout_vec = tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true) ]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_LEFTSHIFT, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, false) ]);
}

// Dual layout, basic test simulating Shift behavior (hold to stay in the second layer),
// but with a key in second layer disabling shift temporarily
fn layered_layout_with_masked_key() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Lhold(1),              G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ G().k(Key::KEY_0).p(),         Inh,           ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), S().m(Key::KEY_LEFTSHIFT).k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        on_active_keys: vec![Key::KEY_LEFTSHIFT],
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}

#[test]
fn test_layered_layout_w_masked_key() {
    let layout_vec = layered_layout_with_masked_key();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    // This temporarily masks the Shift key
    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false), (Key::KEY_E, true), (Key::KEY_E, false), (Key::KEY_LEFTSHIFT, true)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}


// Dual layout, basic test simulating Shift behavior (hold to stay in the second layer),
// but with the second layer disabling active keys on press
fn layered_layout_with_mask() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Lhold(1),              G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ G().k(Key::KEY_0).p(),         Inh,           ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        on_active_keys: vec![Key::KEY_LEFTSHIFT],
        disable_active_on_press: true,
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}


#[test]
fn test_layered_layout_w_mask() {
    let layout_vec = layered_layout_with_mask();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    // This temporarily masks the Shift key
    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false), (Key::KEY_E, true)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, false), (Key::KEY_LEFTSHIFT, true)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_layered_layout_w_mask_crossed() {
    let layout_vec = layered_layout_with_mask();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, true)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t.advance_ms(1));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    // This temporarily masks the Shift key
    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTSHIFT, false), (Key::KEY_E, true)]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, false)]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

// Dual layout, basic test simulating hold layer with timeout behavior
fn hold_and_tap_layered_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ LhtL(1, 2),            G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ No,                    G().k(Key::KEY_T).p(),           ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let keymap_tap = vec![ // blocks
        vec![ // rows
            vec![ No,            G().k(Key::KEY_3).p(), ],
            vec![ G().k(Key::KEY_1).p(), G().k(Key::KEY_2).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let tap_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        keymap: keymap_tap,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer, tap_layer];

    layers
}

#[test]
fn test_hold_and_tap_layered_layout() {
    let layout_vec = hold_and_tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_T, true), (Key::KEY_T, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(190));
    assert_emitted_keys(&mut layout, vec![]);

    // Time was short enough for tap switch
    assert_eq!(layout.get_active_layers(), vec![0, 2]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_2, true), (Key::KEY_2, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_hold_and_tap_layered_layout_long_press() {
    let layout_vec = hold_and_tap_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_T, true), (Key::KEY_T, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(220));
    assert_emitted_keys(&mut layout, vec![]);

    // Time was too long for a tap switch
    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

// Dual layout, basic test simulating hold layer with key timeout behavior
fn hold_and_tap_key_layered_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ LhtK(1, G().k(Key::KEY_0)),   G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ No,                    G().k(Key::KEY_T).p(), ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        keymap: keymap_shift,
        on_active_keys: vec![Key::KEY_4],
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}

#[test]
fn test_hold_and_tap_key_layered_layout() {
    let layout_vec = hold_and_tap_key_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_4, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_T, true), (Key::KEY_T, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    // Time was short enough for tap key
    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(190));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_4, false), (Key::KEY_0, true), (Key::KEY_0, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_hold_and_tap_key_layered_layout_long_press() {
    let layout_vec = hold_and_tap_key_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_4, true)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_T, true), (Key::KEY_T, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    // Time was too long for a tap key
    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(220));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_4, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

// Dual layout, basic test simulating hold layer with key timeout behavior
fn hold_and_tap_keygroup_layered_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ LhtK(1, G().k(Key::KEY_LEFTALT).k(Key::KEY_0)),   G().k(Key::KEY_B).p(), ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(),                          No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ No,                    G().k(Key::KEY_T).p(), ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}

#[test]
fn test_hold_and_tap_keygroup_layered_layout() {
    let layout_vec = hold_and_tap_keygroup_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_T, true), (Key::KEY_T, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    // Time was short enough for tap key
    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(190));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_LEFTALT, true), (Key::KEY_0, true), (Key::KEY_0, false), (Key::KEY_LEFTALT, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_hold_and_tap_keygroup_layered_layout_long_press() {
    let layout_vec = hold_and_tap_keygroup_layered_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_T, true), (Key::KEY_T, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    // Time was too long for a tap key
    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(220));
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

// Singlke layout, basic test simulating short and long presses
fn short_long_press_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Klong(G().k(Key::KEY_0), G().k(Key::KEY_1)),   G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(),           No,           ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer];

    layers
}

#[test]
fn test_short_long_press_layout() {
    let layout_vec = short_long_press_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(200));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_0, true), (Key::KEY_0, false)]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t.advance_ms(100));
    assert_emitted_keys(&mut layout, vec![]);

    // Long press is based on time in the state machine, but also
    // on the state analyzer sending a LongPress event.

    // First long press is not long enough to be detected as long
    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(100));
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(500));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_1, true), (Key::KEY_1, false)]);

    // LongPress might arrive multiple times, additional events should do nothing
    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(500));
    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(200));
    assert_emitted_keys(&mut layout, vec![]);
}

// Dual layout, basic test simulating tap to key, hold to enable layer
fn short_key_long_layer_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Khl(G().k(Key::KEY_0), 1),   G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(),                          No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ No,             G().k(Key::KEY_T).p(), ],
            vec![ Ldeactivate(1), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}

#[test]
fn test_short_key_long_layer_layout() {
    let layout_vec = short_key_long_layer_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    // Time was short enough for tap key
    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(190));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_0, true), (Key::KEY_0, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_short_key_long_layer_layout_long_press() {
    let layout_vec = short_key_long_layer_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    // Time was too long for a tap key
    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(220));
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(100));
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_E, false)]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B03), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0]);
}

// Dual layout, basic test simulating tap to key, hold to enable tap layer
fn short_key_long_tap_layer_layout() -> Vec<Layer> {
    let keymap_default = vec![ // blocks
        vec![ // rows
            vec![ Khtl(G().k(Key::KEY_0), 1),   G().k(Key::KEY_B).p() ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(),                          No,           ],
        ],
    ];

    let keymap_shift = vec![ // blocks
        vec![ // rows
            vec![ No,                    G().k(Key::KEY_T).p(), ],
            vec![ G().k(Key::KEY_LEFTSHIFT).p(), G().k(Key::KEY_E).p(), ],
        ],
    ];

    let default_layer = Layer{
        keymap: keymap_default,
        ..DEFAULT_LAYER_CONFIG
    };

    let shift_layer = Layer{
        status_on_reset: crate::layout::types::LayerStatus::LayerPassthrough,
        keymap: keymap_shift,
        ..DEFAULT_LAYER_CONFIG
    };

    let layers = vec![default_layer, shift_layer];

    layers
}

#[test]
fn test_short_key_long_tap_layer_layout() {
    let layout_vec = short_key_long_tap_layer_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    // Time was short enough for tap key
    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(190));
    assert_emitted_keys(&mut layout, vec![(Key::KEY_0, true), (Key::KEY_0, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

#[test]
fn test_short_key_long_tap_layer_layout_long_press() {
    let layout_vec = short_key_long_tap_layer_layout();
    let mut layout = LayerSwitcher::new(&layout_vec);
    layout.start();
    let mut t = TestTime::start();

    assert_emitted_keys(&mut layout, vec![]);

    layout.process_keyevent(KeyStateChange::Pressed(TestDevice::B01), t);
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B02), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_B, true), (Key::KEY_B, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    // Time was too long for a tap key
    layout.process_keyevent(KeyStateChange::LongPress(TestDevice::B01), t.advance_ms(220));
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Released(TestDevice::B01), t.advance_ms(100));
    assert_emitted_keys(&mut layout, vec![]);

    assert_eq!(layout.get_active_layers(), vec![0, 1]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![(Key::KEY_E, true), (Key::KEY_E, false)]);

    assert_eq!(layout.get_active_layers(), vec![0]);

    layout.process_keyevent(KeyStateChange::Click(TestDevice::B04), t);
    assert_emitted_keys(&mut layout, vec![]);
}

