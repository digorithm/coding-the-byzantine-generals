use rand::{prelude::SliceRandom, Rng};
use std::{
    borrow::Borrow, cell::RefCell, cmp::Ordering, collections::HashMap, hash::Hash, rc::Rc,
    slice::SliceIndex,
};

#[derive(Debug, Clone, Default)]
struct General {
    id: usize,
    messages: Vec<Message>,
    is_traitor: bool,
    decision: bool,
}

impl General {
    fn new(id: usize, is_traitor: bool) -> Self {
        Self {
            id,
            is_traitor,
            // generals: Vec::new(),
            messages: Vec::new(),
            decision: false,
        }
    }

    // Idea to try next: global message list that we pass to all generals
    fn receive_order(&mut self, msg: Message, from: usize) {
        self.messages.push(msg);
        println!(
            "General #{} receiving order ({}) from commander #{}\n",
            self.id, msg.attack, from,
        );
    }

    fn next_order(&self, msg: Message, idx: usize) -> Message {
        if self.is_traitor
        /*&& idx % 2 == 0*/
        {
            if msg.attack {
                return Message { attack: false };
            } else {
                return Message { attack: true };
            }
        }
        msg
    }

    // Given all messages sent to this general, get the consensus
    fn get_local_majority(&self) -> bool {
        let msgs = self
            .messages
            .iter()
            .map(|m| m.attack)
            .collect::<Vec<bool>>();

        let attack = msgs.iter().filter(|&attack| *attack).count();
        let retreat = msgs.iter().filter(|&attack| !*attack).count();

        println!("General #{} orders: {:?}", self.borrow().id, msgs);

        if attack > retreat {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Default, Copy)]
struct Message {
    attack: bool,
}

struct OMAlgorithm {
    is_first_commander_loyal: bool,
    generals: Vec<Rc<RefCell<General>>>,
    original_order: Message,
}

// Note: the goal is that all _loyal_ general decide on the same plan. It's okay for
// the traitors to decide on a different plan.
impl OMAlgorithm {
    fn om_algorithm(&self, commander: Rc<RefCell<General>>, m: usize, msg: Message) {
        // If I'm a traitor, I'm gonna send a random order
        // either the correct or wrong one.
        println!(
            "I'm commander #{}. Sending message: {:?}. m: {}\n",
            commander.as_ref().borrow().id,
            msg.attack,
            m
        );

        // Experimental

        let mut message_deliveries: Vec<(usize, Message)> = vec![];

        if m == 0 {
            // OM(0) (1):
            // The commander sends his value to every lieutenant.
            for (idx, general_rc) in self.generals.iter().enumerate() {
                let general_id = general_rc.as_ref().borrow().id;
                let commander_id = commander.as_ref().borrow().id;

                if general_id != commander_id {
                    let mut general = general_rc.borrow_mut();
                    let msg_to_relay = commander.as_ref().borrow().next_order(msg, idx);
                    general.receive_order(msg_to_relay, commander_id);
                    // message_deliveries.push((general_id, msg_to_relay));
                }
            }

            // OM(0) (2):
            // Each lieutenant uses the value he receives from the commander
            // let commander_id = commander.as_ref().borrow().id;
            // commander.borrow_mut().receive_order(msg, commander_id);
        } else {
            // OM(1) (1):
            // The commander sends his value to every lieutenant.
            for (idx, general_rc) in self.generals.iter().enumerate() {
                let general_id = general_rc.as_ref().borrow().id;
                let commander_id = commander.as_ref().borrow().id;
                if general_id != commander_id {
                    let mut general = general_rc.borrow_mut();

                    let msg_to_relay = commander.as_ref().borrow().next_order(msg, idx);

                    general.receive_order(msg_to_relay, commander.as_ref().borrow().id);
                }
            }

            // OM(1) (1):
            // For each i, let v(i) be the value Lieutenant i receives
            // from the commander, or else be RETREAT if he
            // receives no value.
            // Lieutenant i acts as the commander in Algorithm OM(m-1)
            // to send the value v(i) to each of the n-2 other lieutenants.
            for (idx, general_rc) in self.generals.iter().enumerate() {
                let general = general_rc.as_ref().borrow();
                let commander_id = commander.as_ref().borrow().id;
                if general.id != commander_id {
                    println!("### general #{} acting as commander ###\n", general.id);

                    // let next_order = self.next_order(&general, msg);
                    drop(general);
                    let msg_to_relay = commander.as_ref().borrow().next_order(msg, idx);

                    // Sending message v(i) received from the commander
                    self.om_algorithm(Rc::clone(general_rc), m - 1, msg_to_relay);
                }
            }
        }

        // Deliver messages to generals once we've release the borrow_mut
        // that happens at the recursive step

        // for (id, msg) in message_deliveries {
        //     self.generals
        //         .iter()
        //         .find(|g| g.as_ref().borrow().id == id)
        //         .unwrap()
        //         .borrow_mut()
        //         .messages
        //         .push(msg);
        // }
    }

    fn get_decisions(&self) {
        for general in &self.generals {
            let decision = general.as_ref().borrow().get_local_majority();
            println!(
                "General #{} decision: {}\n",
                general.as_ref().borrow().id,
                decision
            );
        }
    }

    fn was_successful(&self) -> bool {
        // Here's how Lamport defines success:
        // IC1: All loyal lieutenants obey the same order.
        // IC2: If the commanding general is loyal, then every loyal
        // lieutenant obeys the order he sends.

        // Decision of each loyal general
        let mut loyal_decisions = vec![];

        for general in &self.generals {
            let general = general.as_ref().borrow();
            if !general.is_traitor {
                loyal_decisions.push(general.get_local_majority())
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
    // Idea: a good way to explain this is to really focus on the ICs
    // as in, to really focus on what we're trying to achieve here.

    let num_of_generals = 4;
    let num_of_traitors = 2;
    let m = num_of_traitors;

    let num_of_experiments = 10;

    for _i in 0..num_of_experiments {
        let mut generals: Vec<Rc<RefCell<General>>> = vec![];

        for i in 0..num_of_generals {
            let g = Rc::new(RefCell::new(General::new(i + 1, false)));
            generals.push(g);
        }

        let g: Vec<_> = generals
            .choose_multiple(&mut rand::thread_rng(), num_of_traitors)
            .collect();

        for traitor in g {
            println!("Traitor: General #{}\n", traitor.as_ref().borrow().id);
            traitor.as_ref().borrow_mut().is_traitor = true;
        }

        let order = Message { attack: true };

        let mut om_algorithm = OMAlgorithm {
            generals,
            is_first_commander_loyal: true,
            original_order: order,
        };

        let first_commander = rand::thread_rng().gen_range(0..num_of_generals);

        let first_commander_rc = Rc::clone(&om_algorithm.generals[first_commander]);

        om_algorithm.generals.remove(first_commander);

        if first_commander_rc.as_ref().borrow().is_traitor {
            om_algorithm.is_first_commander_loyal = false
        }

        om_algorithm.om_algorithm(first_commander_rc, m, order);

        om_algorithm.get_decisions();

        let successful = om_algorithm.was_successful();

        println!("successful: {:?}\n", successful);
        println!("\n####################################\n");

        if !successful {
            println!("### Found a case where consensus isn't achieved ###");
            break;
        }
    }
}
