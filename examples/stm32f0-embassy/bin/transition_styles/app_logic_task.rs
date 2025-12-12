use defmt::info;
use rgb_sequencer::{SequencerAction16, SequencerCommand16};

use crate::blink_task::BLINK_COUNT_SIGNAL;
use crate::sequences::create_transition_sequence;
use crate::types::{BUTTON_SIGNAL, RGB_COMMAND_CHANNEL, TransitionMode};

#[embassy_executor::task]
pub async fn app_logic_task() {
    info!("App logic task started");

    let mut current_mode = TransitionMode::Step;

    let initial_sequence = create_transition_sequence(current_mode.to_transition_style());
    RGB_COMMAND_CHANNEL
        .send(SequencerCommand16::new(
            (),
            SequencerAction16::Load(initial_sequence),
        ))
        .await;

    BLINK_COUNT_SIGNAL.signal(current_mode.blink_count());

    info!("Initial mode loaded: {:?}", current_mode);

    loop {
        BUTTON_SIGNAL.wait().await;

        current_mode = current_mode.next();
        info!("Switching to mode: {:?}", current_mode);

        BLINK_COUNT_SIGNAL.signal(current_mode.blink_count());

        let sequence = create_transition_sequence(current_mode.to_transition_style());
        RGB_COMMAND_CHANNEL
            .send(SequencerCommand16::new(
                (),
                SequencerAction16::Load(sequence),
            ))
            .await;
    }
}
