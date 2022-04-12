use keyberon::layout::{layout, Layers};
pub enum CustomAction {
    Reset,
    Bootsel,
}

pub static LAYERS: Layers<CustomAction> = layout! {
    {
        [ A B C D E F ]
        [ G H I J K L ]
        [ M N O P Q R ]
        [ S T U n n n ]
    }
};
