//! built-in device layouts the fake can report. the trees are fixed and also
//! serve as test fixtures.

use serde_json::{json, Value};
use sextant_core::component::kind;

/// a fixed eight-component layout: a base with three dials, two buttons, and two
/// sliders.
pub fn expertkit() -> Value {
    json!({"u":"00000","i":1,"t":kind::BASE,"c":[
        null,
        {"u":"  49d","i":2,"t":kind::DIAL,"c":[
            {"u":"  4FH","i":3,"t":kind::DIAL,"c":[
                {"u":"  4lf","i":8,"t":kind::BUTTON,"c":[null,null,null]},
                {"u":"  4}=","i":4,"t":kind::SLIDER,"c":[null,null,null,null,null]},
                null
            ]},
            null,
            {"u":"  4A0","i":5,"t":kind::DIAL,"c":[
                null,
                null,
                {"u":"  4ls","i":6,"t":kind::BUTTON,"c":[
                    {"u":"  4}M","i":7,"t":kind::SLIDER,"c":[null,null,null,null,null]},
                    null,
                    null
                ]}
            ]}
        ]},
        null
    ]})
}

/// a fixed fifteen-component layout exercising every kind and orientation.
pub fn prokit() -> Value {
    json!({"u":"00000","i":1,"t":kind::BASE,"c":[
        null,
        {"u":"  C4@","i":2,"t":kind::DIAL,"c":[
            {"u":"  D5o","i":3,"t":kind::DIAL,"c":[
                {"u":"  B$;","i":4,"t":kind::BUTTON,"c":[
                    null,
                    {"u":"  8S$","i":5,"t":kind::BUTTON,"c":[null,null,null]},
                    null
                ]},
                {"u":"  Ak~","i":11,"t":kind::SLIDER,"c":[
                    null,
                    {"u":"  6om","i":12,"t":kind::DIAL,"c":[null,null,null]},
                    null,
                    null,
                    null
                ]},
                {"u":"  8Uq","i":13,"t":kind::SLIDER,"c":[null,null,null,null,null]}
            ]},
            {"u":"  6pZ","i":6,"t":kind::DIAL,"c":[null,null,null]},
            {"u":"  6RD","i":7,"t":kind::DIAL,"c":[
                null,
                null,
                {"u":"  8a4","i":8,"t":kind::BUTTON,"c":[
                    {"u":"  8W(","i":9,"t":kind::SLIDER,"c":[
                        null,
                        {"u":"  8V$","i":10,"t":kind::SLIDER,"c":[null,null,null,null,null]},
                        null,
                        null,
                        null
                    ]},
                    {"u":"  8R+","i":14,"t":kind::BUTTON,"c":[
                        {"u":"  6J@","i":15,"t":kind::DIAL,"c":[null,null,null]},
                        null,
                        null
                    ]},
                    null
                ]}
            ]}
        ]},
        null
    ]})
}

/// a base with no components attached.
pub fn base_only() -> Value {
    json!({"u":"00000","i":1,"t":kind::BASE,"c":[null,null,null]})
}

/// the monogram cc layout: a core (with the screen) connected to a tri-dial and
/// an orbiter, with the device's 0-based indices. mirrors the tree a real
/// monogram core reports, so the gui can render the right shape.
pub fn monogram() -> Value {
    json!({"u":"00000","i":0,"t":kind::CORE,"c":[null,null,null,
        {"u":"00001","i":1,"t":kind::TRI_DIAL,"c":[null,null,null,
            {"u":"00002","i":2,"t":kind::ORBITER,"c":[null,null,null,null,null,null,null]},
            null]},
        null]})
}
