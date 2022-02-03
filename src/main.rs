use rand::{prelude::SliceRandom, Rng};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Default)]
struct General {
    id: usize,
    messages: Vec<Message>,
    is_traitor: bool,
    decision: bool,
    total_messages_received: usize,
}

impl General {
    fn new(id: usize, is_traitor: bool) -> Self {
        Self {
            id,
            is_traitor,
            messages: Vec::new(),
            decision: false,
            total_messages_received: 0,
        }
    }

    fn receive_order(&mut self, msg: Message, from: usize) {
        if self.messages.is_empty() {
            println!(
                "General #{}'s first order received: {}",
                self.id, msg.attack
            );
            self.decision = msg.attack;
        }

        self.messages.push(msg);
        println!(
            "General #{} receiving order ({}) from commander #{}\n",
            self.id, msg.attack, from,
        );
        self.total_messages_received += 1;
    }

    fn next_order(&self, idx: usize) -> Message {
        if self.is_traitor && idx % 2 == 0 {
            if self.decision {
                return Message { attack: false };
            } else {
                return Message { attack: true };
            }
        }
        Message {
            attack: self.decision,
        }
    }

    fn decide(&mut self) {
        let msgs = self
            .messages
            .iter()
            .map(|m| m.attack)
            .collect::<Vec<bool>>();

        println!("Deciding majority for general #{}: {:?}", self.id, msgs);

        let attack = msgs.iter().filter(|&attack| *attack).count();
        let retreat = msgs.iter().filter(|&attack| !*attack).count();

        if attack > retreat {
            self.decision = true;
            return;
        }
        self.decision = false;
    }
}

#[derive(Debug, Clone, Default, Copy)]
struct Message {
    attack: bool,
}

struct OMAlgorithm {
    is_first_commander_loyal: bool,
    original_order: Message,
}

// Note: the goal is that all _loyal_ general decide on the same plan. It's okay for
// the traitors to decide on a different plan.
impl OMAlgorithm {
    fn om_algorithm(
        &self,
        generals: &[Rc<RefCell<General>>],
        commander: Rc<RefCell<General>>,
        m: usize,
    ) {
        println!(
            "I'm commander #{}. Sending message: {:?}. m: {}\n",
            commander.as_ref().borrow().id,
            commander.as_ref().borrow().decision,
            m
        );

        if m == 0 {
            // OM(0):
            // The commander sends his value to every lieutenant.
            for (idx, general_rc) in generals.iter().enumerate() {
                let msg_to_relay = commander.as_ref().borrow().next_order(idx);

                let mut general = general_rc.borrow_mut();

                general.receive_order(msg_to_relay, commander.as_ref().borrow().id);
            }
        } else {
            // OM(1) (1):
            // The commander sends his value to every lieutenant.
            for (idx, general_rc) in generals.iter().enumerate() {
                let msg_to_relay = commander.as_ref().borrow().next_order(idx);

                let mut general = general_rc.borrow_mut();

                general.receive_order(msg_to_relay, commander.as_ref().borrow().id);
            }

            // OM(1) (2):
            // For each i, let v(i) be the value Lieutenant i receives
            // from the commander, or else be RETREAT if he
            // receives no value.
            // Lieutenant i acts as the commander in Algorithm OM(m-1)
            // to send the value v(i) to each of the n-2 other lieutenants.
            for general_rc in generals {
                let next_commander = general_rc.as_ref().borrow();

                println!(
                    "### general #{} acting as commander ###\n",
                    next_commander.id
                );

                let mut new_generals: Vec<Rc<RefCell<General>>> = vec![];

                for general in generals {
                    if general.as_ref().borrow().id != next_commander.id {
                        new_generals.push(Rc::clone(general));
                    }
                }
                drop(next_commander);

                // Sending message v(i) received from the commander
                self.om_algorithm(&new_generals, Rc::clone(general_rc), m - 1);
            }

            // (3) For each i, and each j != i, let v(j) be the value Lieutenant i
            // received from Lieutenant j in step (2) using Algorithm OM(m - 1),
            // or else RETREAT if he received no such value.
            // Lieutenant i uses the value majority (vl, ..., vn-1 ).
            // Note: honestly, this was the most subtle part of the whole algorithm...
            // Each general is updating their beliefs based on what they heard so far.
            for general in generals {
                println!("deciding for general #{}", general.as_ref().borrow().id);
                general.borrow_mut().decide();
            }
        }

        println!(
            "General #{} decision after round OM({}): {}\n",
            commander.as_ref().borrow().id,
            m,
            commander.as_ref().borrow().decision
        );
    }

    fn get_total_messages(&self, generals: &[Rc<RefCell<General>>]) {
        let mut total_messages = 0;
        for general in generals {
            total_messages += general.as_ref().borrow().total_messages_received;
        }

        println!("total_messages: {:?}\n", total_messages);
    }

    fn was_successful(&self, generals: &[Rc<RefCell<General>>]) -> bool {
        // Here's how Lamport defines success:
        // IC1: All loyal lieutenants obey the same order.
        // IC2: If the commanding general is loyal, then every loyal
        // lieutenant obeys the order he sends.

        // Decision of each loyal general
        let mut loyal_decisions = vec![];

        for general in generals {
            let general = general.as_ref().borrow();
            if !general.is_traitor {
                loyal_decisions.push(general.decision)
            }
        }

        // Was there consensus amongst all loyal generals?
        let loyal_consensus = loyal_decisions.windows(2).all(|w| w[0] == w[1]);

        // If not, we've broken IC1
        if !loyal_consensus {
            return false;
        }

        // If we had consensus among loyal generals and the first commander
        // is loyal, does the consensus match the original order?
        // If it doesn't, we've broken IC2.
        if self.is_first_commander_loyal {
            // Then the loyal consensus should match original order
            if self.original_order.attack != loyal_decisions[0] {
                return false;
            }
        }

        // IC1 and IC2 hold
        true
    }
}

fn main() {
    // Configuration params
    let num_of_generals = 4;
    let num_of_traitors = 1;
    let m = num_of_traitors;
    let num_of_experiments = 10;

    for _i in 0..num_of_experiments {
        let mut generals: Vec<Rc<RefCell<General>>> = vec![];

        // Create all generals, set `is_traitor` as false initially
        for i in 0..num_of_generals {
            let g = Rc::new(RefCell::new(General::new(i + 1, false)));
            generals.push(g);
        }

        // Pick `num_of_traitors` random generals and set them as traitors
        // Note that this could also be the first commanding general
        generals
            .choose_multiple(&mut rand::thread_rng(), num_of_traitors)
            .for_each(|traitor| {
                println!("Traitor: General #{}\n", traitor.as_ref().borrow().id);
                traitor.as_ref().borrow_mut().is_traitor = true;
            });

        let order = Message { attack: true };

        let mut om_algorithm = OMAlgorithm {
            is_first_commander_loyal: true,
            original_order: order,
        };

        // Randomly pick first commanding general
        let first_commander = rand::thread_rng().gen_range(0..num_of_generals);
        let first_commander_rc = Rc::clone(&generals[first_commander]);

        first_commander_rc.borrow_mut().decision = order.attack;

        // Remove first commanding general out of the initial list of lieutenant generals
        generals.remove(first_commander);

        if first_commander_rc.as_ref().borrow().is_traitor {
            om_algorithm.is_first_commander_loyal = false
        }

        // Start the algorithm
        om_algorithm.om_algorithm(&generals, first_commander_rc, m);

        // Calculate total messages sent for analytical purposes
        om_algorithm.get_total_messages(&generals);

        // Check whether it was successful
        let successful = om_algorithm.was_successful(&generals);

        println!("successful: {:?}\n", successful);

        if !successful {
            println!("Found a case where consensus isn't achieved\n");
            break;
        }
    }
}
