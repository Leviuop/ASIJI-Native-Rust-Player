//! Request millisecond waits only while the Windows player is active.
pub struct PlaybackTimer;

#[cfg(windows)]
#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(period: u32) -> u32;
    fn timeEndPeriod(period: u32) -> u32;
}

impl PlaybackTimer {
    pub fn start() -> anyhow::Result<Self> {
        #[cfg(windows)]
        // SAFETY: both WinMM functions accept an integer resolution, with no pointers.
        anyhow::ensure!(
            unsafe { timeBeginPeriod(1) } == 0,
            "Не удалось включить точный таймер Windows"
        );
        Ok(Self)
    }
}

impl Drop for PlaybackTimer {
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: balances the successful request made by start().
        unsafe {
            timeEndPeriod(1);
        }
    }
}
