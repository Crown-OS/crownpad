crownos_ipc::protocol! {
    pub mod protocol: "crownpad" {
        fn show() -> bool;
        fn hide() -> bool;
        fn toggle() -> bool;
        fn is_visible() -> bool;
    }
}
