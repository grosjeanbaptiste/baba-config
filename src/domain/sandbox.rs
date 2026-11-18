//! Sandboxed execution policies and profile generation (ADR-040).
//!
//! Confinement policies for Baba actions based on the domain taxonomy
//! ([`VerbClass`]):
//! - `ReadOnly`: pure queries, denies network and all file writes except stdio.
//! - `WorkspaceConfined`: allows writes only to specified directories (and tempdirs),
//!   blocks network unless `allow_network` is true.
//! - `Disabled`: bypasses confinement entirely.

use std::fmt::Write;
use std::path::PathBuf;

use super::taxonomy::VerbClass;

/// Confinement policy for executing commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SandboxPolicy {
    /// Pure query — no network, read-only filesystem (except stdio/null).
    ReadOnly,
    /// Mutating / scoped — writes restricted to specific directories + tempdirs.
    WorkspaceConfined {
        /// Directories where write access is permitted.
        allowed_write_dirs: Vec<PathBuf>,
        /// Whether outbound network sockets are permitted.
        allow_network: bool,
    },
    /// Confinement is disabled (passthrough).
    Disabled,
}

impl SandboxPolicy {
    /// Derive a policy from an optional [`VerbClass`].
    /// Fail-safe: unknown / missing class becomes `WorkspaceConfined` without network.
    #[must_use]
    pub fn for_verb_class(class: Option<VerbClass>, target_dirs: &[PathBuf]) -> Self {
        match class {
            Some(VerbClass::Readonly) => Self::ReadOnly,
            Some(VerbClass::Outward) => Self::WorkspaceConfined {
                allowed_write_dirs: target_dirs.to_vec(),
                allow_network: true,
            },
            Some(VerbClass::Destructive) | None => Self::WorkspaceConfined {
                allowed_write_dirs: target_dirs.to_vec(),
                allow_network: false,
            },
        }
    }

    /// Whether this policy imposes confinement.
    #[must_use]
    pub fn is_confined(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Generate a macOS Seatbelt (sandbox-exec) profile string.
    /// Returns `None` if sandboxing is `Disabled`.
    #[must_use]
    pub fn to_seatbelt_profile(&self) -> Option<String> {
        match self {
            Self::Disabled => None,
            Self::ReadOnly => Some(String::from(
                "(version 1)\n\
                 (deny default)\n\
                 (allow process-exec)\n\
                 (allow process-fork)\n\
                 (allow sysctl-read)\n\
                 (allow file-read*)\n\
                 (allow signal (target self))\n\
                 (allow ipc-posix-shm)\n\
                 (allow file-write-data (literal \"/dev/null\") (literal \"/dev/zero\") (literal \"/dev/tty\"))\n\
                 (allow file-ioctl (literal \"/dev/null\") (literal \"/dev/zero\") (literal \"/dev/tty\") (literal \"/dev/dtracehelper\"))\n",
            )),
            Self::WorkspaceConfined {
                allowed_write_dirs,
                allow_network,
            } => {
                let mut p = String::from(
                    "(version 1)\n\
                     (deny default)\n\
                     (allow process-exec)\n\
                     (allow process-fork)\n\
                     (allow sysctl-read)\n\
                     (allow file-read*)\n\
                     (allow signal (target self))\n\
                     (allow ipc-posix-shm)\n\
                     (allow file-ioctl (literal \"/dev/null\") (literal \"/dev/zero\") (literal \"/dev/tty\") (literal \"/dev/dtracehelper\"))\n\
                     (allow file-write*\n\
                       (literal \"/dev/null\")\n\
                       (literal \"/dev/zero\")\n\
                       (literal \"/dev/tty\")\n\
                       (subpath \"/tmp\")\n\
                       (subpath \"/private/tmp\")\n",
                );
                for dir in allowed_write_dirs {
                    let path_str = dir.to_string_lossy();
                    let _ = writeln!(p, "  (subpath \"{path_str}\")");
                }
                p.push_str(")\n");
                if *allow_network {
                    p.push_str("(allow network*)\n");
                }
                Some(p)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_from_verb_class_maps_correctly() {
        let ws = vec![PathBuf::from("/workspace")];
        assert_eq!(
            SandboxPolicy::for_verb_class(Some(VerbClass::Readonly), &ws),
            SandboxPolicy::ReadOnly
        );
        assert_eq!(
            SandboxPolicy::for_verb_class(Some(VerbClass::Destructive), &ws),
            SandboxPolicy::WorkspaceConfined {
                allowed_write_dirs: ws.clone(),
                allow_network: false,
            }
        );
        assert_eq!(
            SandboxPolicy::for_verb_class(Some(VerbClass::Outward), &ws),
            SandboxPolicy::WorkspaceConfined {
                allowed_write_dirs: ws.clone(),
                allow_network: true,
            }
        );
        // Fail-safe for unknown/unclassified verb
        assert_eq!(
            SandboxPolicy::for_verb_class(None, &ws),
            SandboxPolicy::WorkspaceConfined {
                allowed_write_dirs: ws,
                allow_network: false,
            }
        );
    }

    #[test]
    fn is_confined_is_true_unless_disabled() {
        assert!(SandboxPolicy::ReadOnly.is_confined());
        assert!(
            SandboxPolicy::WorkspaceConfined {
                allowed_write_dirs: vec![],
                allow_network: false,
            }
            .is_confined()
        );
        assert!(!SandboxPolicy::Disabled.is_confined());
    }

    #[test]
    fn seatbelt_profile_readonly() {
        let profile = SandboxPolicy::ReadOnly
            .to_seatbelt_profile()
            .expect("profile");
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(allow file-read*)"));
        assert!(profile.contains("(allow process-exec)"));
        assert!(!profile.contains("(allow network*)"));
        assert!(!profile.contains("(allow file-write*"));
    }

    #[test]
    fn seatbelt_profile_workspace_confined_without_network() {
        let policy = SandboxPolicy::WorkspaceConfined {
            allowed_write_dirs: vec![PathBuf::from("/my/repo")],
            allow_network: false,
        };
        let profile = policy.to_seatbelt_profile().expect("profile");
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(allow file-read*)"));
        assert!(profile.contains("(allow file-write*"));
        assert!(profile.contains("(subpath \"/my/repo\")"));
        assert!(profile.contains("(subpath \"/tmp\")"));
        assert!(!profile.contains("(allow network*)"));
    }

    #[test]
    fn seatbelt_profile_workspace_confined_with_network() {
        let policy = SandboxPolicy::WorkspaceConfined {
            allowed_write_dirs: vec![PathBuf::from("/my/repo")],
            allow_network: true,
        };
        let profile = policy.to_seatbelt_profile().expect("profile");
        assert!(profile.contains("(allow network*)"));
        assert!(profile.contains("(subpath \"/my/repo\")"));
    }

    #[test]
    fn seatbelt_profile_disabled_returns_none() {
        assert_eq!(SandboxPolicy::Disabled.to_seatbelt_profile(), None);
    }
}
