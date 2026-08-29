use std::process::Command;

#[cfg(target_os = "macos")]
mod mac {
    use super::Command;
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(
            options: core_foundation::dictionary::CFDictionaryRef,
        ) -> bool;
    }

    pub fn accessibility_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn request_accessibility() -> Result<(), String> {
        let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let dict = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        unsafe {
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef());
        }
        let _ = Command::new("open")
            .arg("x-apple.systemsettings:com.apple.preference.security?Privacy_Accessibility")
            .status();
        Ok(())
    }

    const PLAYERS: [&str; 2] = ["Spotify", "Music"];
    const BROWSERS: [&str; 5] = [
        "Safari",
        "Google Chrome",
        "Google Chrome Canary",
        "Brave Browser",
        "Arc",
    ];

    pub fn request_automation() -> Result<(), String> {
        let _ = super::super::macos::osascript_consent(
            r#"tell application "System Events" to get name of first process"#,
        );
        for app in PLAYERS {
            if app_exists(app) {
                prompt(app);
            }
        }

        for app in BROWSERS {
            if app_running(app) {
                prompt(app);
            }
        }
        Ok(())
    }

    fn prompt(name: &str) {
        let _ = super::super::macos::osascript_consent(&format!(
            r#"tell application "{name}" to get name"#
        ));
    }

    fn osascript_bool(source: &str) -> bool {
        Command::new("osascript")
            .args(["-e", source])
            .output()
            .ok()
            .is_some_and(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .eq_ignore_ascii_case("true")
            })
    }

    fn app_exists(name: &str) -> bool {
        osascript_bool(&format!("exists application \"{name}\""))
    }

    fn app_running(name: &str) -> bool {
        app_exists(name) && osascript_bool(&format!("application \"{name}\" is running"))
    }
}

pub fn accessibility_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::accessibility_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn request_accessibility() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        mac::request_accessibility()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

pub fn request_automation() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        mac::request_automation()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
