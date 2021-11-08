use crate::interaction_handlers::picking_session::Team;

/// Generate an ordered list of team colors.
///
/// For 2 player game modes this simply determines who ends up on blue/red team (random).
///
/// For game modes with more players, it determines which captain picks first
/// and is used to validate picking order/turns when choosing players from the roster.
pub fn generate(player_count: &u64) -> Vec<Team> {
    let player_count = *player_count;
    let mut pick_sequence: Vec<Team>;
    let options = [Team::Blue, Team::Red];
    let random_first_pick = &options[rand::Rng::gen_range(&mut rand::thread_rng(), 0..2)];
    match random_first_pick {
        Team::Blue => {
            pick_sequence = vec![Team::Blue];
        }
        Team::Red => {
            pick_sequence = vec![Team::Red];
        }
    }

    // loop should only operate if game mode is for more than 2 players
    if player_count > 2 {
        // 2 is ACTUALLY the second index and not third
        // Since the player count is 1-based,
        // the counter for this loop is also 1-based for the sake consistency

        let mut iter = 2..player_count;
        for _ in iter.step_by(2) {
            // This loop operates on the indexes between the first and last
            // Captains alternate double picks when its not first/last pick round,
            // so this loop inserts the double pick turns for all the
            // picking rounds inbetween the first and last pick

            match pick_sequence.last().unwrap() {
                Team::Blue => {
                    pick_sequence.push(Team::Red);
                    pick_sequence.push(Team::Red);
                }
                Team::Red => {
                    pick_sequence.push(Team::Blue);
                    pick_sequence.push(Team::Blue);
                }
            }
        }
    }

    let blue_count = pick_sequence.iter().filter(|&p| *p == Team::Blue).count();
    let red_count = pick_sequence.iter().filter(|&p| *p == Team::Red).count();

    // the variant with lower occurences in the sequence fills the last spot in the vec
    if blue_count < red_count {
        pick_sequence.push(Team::Blue);
    } else {
        pick_sequence.push(Team::Red);
    }

    pick_sequence
}
