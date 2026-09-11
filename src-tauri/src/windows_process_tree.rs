fn descendants(root: u32, processes: &[(u32, u32)]) -> Vec<u32> {
    let mut found = vec![root];
    let mut seen = std::collections::BTreeSet::from([root]);
    let mut index = 0;
    while index < found.len() {
        let parent = found[index];
        for &(pid, ppid) in processes {
            if ppid == parent && seen.insert(pid) {
                found.push(pid);
            }
        }
        index += 1;
    }
    found
        .into_iter()
        .rev()
        .filter(|pid| processes.iter().any(|&(existing, _)| existing == *pid))
        .collect()
}

#[cfg(windows)]
fn snapshot() -> Result<Vec<(u32, u32)>, String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    // Handle and initialized entry remain valid through enumeration; close the handle once.
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle == INVALID_HANDLE_VALUE {
            return Err("Cannot inspect download subprocesses.".into());
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut processes = Vec::new();
        let mut valid = Process32FirstW(handle, &mut entry);
        while valid != 0 {
            processes.push((entry.th32ProcessID, entry.th32ParentProcessID));
            valid = Process32NextW(handle, &mut entry);
        }
        CloseHandle(handle);
        Ok(processes)
    }
}

#[cfg(windows)]
pub(crate) fn kill(root: u32) -> Result<(), String> {
    // Descendants retain their parent ID after exit; taskkill /T on a dead root does not.
    for pid in descendants(root, &snapshot()?) {
        let output = crate::background_command("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map_err(crate::to_string)?;
        if !output.status.success() && snapshot()?.iter().any(|&(existing, _)| existing == pid) {
            return Err(crate::process_failure_message(
                "Failed to stop download subprocess.",
                output.status.code(),
                &output.stderr,
                &output.stdout,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_descendants_even_when_parent_has_already_exited() {
        assert_eq!(descendants(7, &[(9, 7), (11, 9), (88, 8)]), vec![11, 9]);
        assert_eq!(
            descendants(7, &[(7, 1), (9, 7), (11, 9), (88, 8)]),
            vec![11, 9, 7]
        );
        assert!(descendants(7, &[(88, 8)]).is_empty());
    }
}
