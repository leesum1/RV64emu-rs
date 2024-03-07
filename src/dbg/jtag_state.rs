use log::trace;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum JtagState {
    TestLogicReset,
    RunTestIdle,
    SelectDrScan,
    CaptureDr,
    ShiftDr,
    Exit1Dr,
    PauseDr,
    Exit2Dr,
    UpdateDr,
    SelectIrScan,
    CaptureIr,
    ShiftIr,
    Exit1Ir,
    PauseIr,
    Exit2Ir,
    UpdateIr,
}

impl JtagState {
    pub fn is_update_ir(&self) -> bool {
        use JtagState::*;
        matches!(self, UpdateIr)
    }

    pub fn is_update_dr(&self) -> bool {
        use JtagState::*;
        matches!(self, UpdateDr)
    }

    pub fn is_capture_ir(&self) -> bool {
        use JtagState::*;
        matches!(self, CaptureIr)
    }

    pub fn is_capture_dr(&self) -> bool {
        use JtagState::*;
        matches!(self, CaptureDr)
    }

    pub fn next_state(&self, tms: bool) -> JtagState {
        use JtagState::*;
        let state = match self {
            TestLogicReset => {
                if tms {
                    TestLogicReset
                } else {
                    RunTestIdle
                }
            }
            RunTestIdle => {
                if tms {
                    SelectDrScan
                } else {
                    RunTestIdle
                }
            }
            SelectDrScan => {
                if tms {
                    SelectIrScan
                } else {
                    CaptureDr
                }
            }
            CaptureDr => {
                if tms {
                    Exit1Dr
                } else {
                    ShiftDr
                }
            }
            ShiftDr => {
                if tms {
                    Exit1Dr
                } else {
                    ShiftDr
                }
            }
            Exit1Dr => {
                if tms {
                    UpdateDr
                } else {
                    PauseDr
                }
            }
            PauseDr => {
                if tms {
                    Exit2Dr
                } else {
                    PauseDr
                }
            }
            Exit2Dr => {
                if tms {
                    UpdateDr
                } else {
                    ShiftDr
                }
            }
            UpdateDr => {
                if tms {
                    SelectDrScan
                } else {
                    RunTestIdle
                }
            }
            SelectIrScan => {
                if tms {
                    TestLogicReset
                } else {
                    CaptureIr
                }
            }
            CaptureIr => {
                if tms {
                    Exit1Ir
                } else {
                    ShiftIr
                }
            }
            ShiftIr => {
                if tms {
                    Exit1Ir
                } else {
                    ShiftIr
                }
            }
            Exit1Ir => {
                if tms {
                    UpdateIr
                } else {
                    PauseIr
                }
            }
            PauseIr => {
                if tms {
                    Exit2Ir
                } else {
                    PauseIr
                }
            }
            Exit2Ir => {
                if tms {
                    UpdateIr
                } else {
                    ShiftIr
                }
            }
            UpdateIr => {
                if tms {
                    SelectDrScan
                } else {
                    RunTestIdle
                }
            }
        };

        // trace!("tms:{},State transition: {:?} -> {:?}", tms, self, state);

        state
    }
}

#[cfg(test)]
mod jtag_state_tests {
    use super::*;

    #[test]
    fn reset_test() {
        for state in &[
            JtagState::TestLogicReset,
            JtagState::RunTestIdle,
            JtagState::SelectDrScan,
            JtagState::CaptureDr,
            JtagState::ShiftDr,
            JtagState::Exit1Dr,
            JtagState::PauseDr,
            JtagState::Exit2Dr,
            JtagState::UpdateDr,
            JtagState::SelectIrScan,
            JtagState::CaptureIr,
            JtagState::ShiftIr,
            JtagState::Exit1Ir,
            JtagState::PauseIr,
            JtagState::Exit2Ir,
            JtagState::UpdateIr,
        ] {
            let mut state = *state;
            for _ in 0..5 {
                state = state.next_state(true);
            }
            assert_eq!(state, JtagState::TestLogicReset);
            println!("Reset test passed for state: {:?}\n", state);
        }
    }
}
