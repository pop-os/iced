use cfg_aliases::cfg_aliases;

pub fn cfg_aliases_setup() {
    // The script doesn't depend on our code
    println!("cargo:rerun-if-changed=build.rs");

    // Setup cfg aliases
    cfg_aliases! {
        // Systems.
        android_platform: { target_os = "android" },
        wasm_platform: { all(target_family = "wasm", not(target_os = "emscripten")) },
        macos_platform: { target_os = "macos" },
        ios_platform: { target_os = "ios" },
        windows_platform: { target_os = "windows" },
        apple: { any(target_os = "ios", target_os = "macos") },
        free_unix: { all(unix, not(apple), not(android_platform), not(target_os = "emscripten")) },
        redox: { target_os = "redox" },

        // Native displays.
        wayland_platform: { all(feature = "wayland", free_unix, not(wasm), not(redox)) },
        orbital_platform: { redox },
    }
}
