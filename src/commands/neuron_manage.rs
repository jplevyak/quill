use crate::{
    commands::{request_status, sign::sign},
    lib::{
        nns_types::{account_identifier::AccountIdentifier, icpts::ICPTs},
        AnyhowResult, GOVERNANCE_CANISTER_ID,
    },
};
use anyhow::anyhow;
use candid::{CandidType, Encode};
use clap::Clap;
use ic_types::Principal;

#[derive(CandidType)]
pub struct IncreaseDissolveDelay {
    pub additional_dissolve_delay_seconds: u32,
}

#[derive(CandidType)]
pub struct NeuronId {
    pub id: u64,
}

#[derive(CandidType)]
pub struct StartDissolving {}

#[derive(CandidType)]
pub struct StopDissolving {}

#[derive(CandidType)]
pub struct RemoveHotKey {
    pub hot_key_to_remove: Option<Principal>,
}

#[derive(CandidType)]
pub struct AddHotKey {
    pub new_hot_key: Option<Principal>,
}

#[derive(CandidType)]
pub enum Operation {
    RemoveHotKey(RemoveHotKey),
    StartDissolving(StartDissolving),
    StopDissolving(StopDissolving),
    AddHotKey(AddHotKey),
    IncreaseDissolveDelay(IncreaseDissolveDelay),
}

#[derive(CandidType)]
pub struct Configure {
    pub operation: Option<Operation>,
}

#[derive(CandidType)]
pub struct Disburse {
    pub to_account: Option<AccountIdentifier>,
    pub amount: Option<ICPTs>,
}

#[derive(CandidType)]
pub enum Command {
    Configure(Configure),
    Disburse(Disburse),
}

#[derive(CandidType)]
struct ManageNeuron {
    id: Option<NeuronId>,
    command: Option<Command>,
}

/// Signs a neuron configuration change.
#[derive(Clap)]
pub struct ManageOpts {
    neuron_id: u64,

    /// Principal to be used as a hot key.
    #[clap(long)]
    add_hot_key: Option<Principal>,

    /// Principal hot key to be removed.
    #[clap(long)]
    remove_hot_key: Option<Principal>,

    /// Amount of dissolve seconds to add.
    #[clap(short, long)]
    additional_dissolve_delay_seconds: Option<u32>,

    /// Start dissolving.
    #[clap(long)]
    start_dissolving: bool,

    /// Stop dissolving.
    #[clap(long)]
    stop_dissolving: bool,

    /// Disburse the entire staked amount to the controller's account.
    #[clap(long)]
    disburse: bool,
}

pub async fn exec(pem: &Option<String>, opts: ManageOpts) -> AnyhowResult<String> {
    let mut msgs = Vec::new();

    if opts.add_hot_key.is_some() {
        let args = Encode!(&ManageNeuron {
            id: Some(NeuronId { id: opts.neuron_id }),
            command: Some(Command::Configure(Configure {
                operation: Some(Operation::AddHotKey(AddHotKey {
                    new_hot_key: opts.add_hot_key
                }))
            }))
        })?;
        msgs.push(generate(pem, args).await?);
    };

    if opts.remove_hot_key.is_some() {
        let args = Encode!(&ManageNeuron {
            id: Some(NeuronId { id: opts.neuron_id }),
            command: Some(Command::Configure(Configure {
                operation: Some(Operation::RemoveHotKey(RemoveHotKey {
                    hot_key_to_remove: opts.remove_hot_key
                }))
            }))
        })?;
        msgs.push(generate(pem, args).await?);
    };

    if opts.stop_dissolving {
        let args = Encode!(&ManageNeuron {
            id: Some(NeuronId { id: opts.neuron_id }),
            command: Some(Command::Configure(Configure {
                operation: Some(Operation::StopDissolving(StopDissolving {}))
            }))
        })?;
        msgs.push(generate(pem, args).await?);
    }

    if opts.start_dissolving {
        let args = Encode!(&ManageNeuron {
            id: Some(NeuronId { id: opts.neuron_id }),
            command: Some(Command::Configure(Configure {
                operation: Some(Operation::StartDissolving(StartDissolving {}))
            }))
        })?;
        msgs.push(generate(pem, args).await?);
    }

    if let Some(additional_dissolve_delay_seconds) = opts.additional_dissolve_delay_seconds {
        let args = Encode!(&ManageNeuron {
            id: Some(NeuronId { id: opts.neuron_id }),
            command: Some(Command::Configure(Configure {
                operation: Some(Operation::IncreaseDissolveDelay(IncreaseDissolveDelay {
                    additional_dissolve_delay_seconds
                }))
            }))
        })?;
        msgs.push(generate(pem, args).await?);
    };

    if opts.disburse {
        let args = Encode!(&ManageNeuron {
            id: Some(NeuronId { id: opts.neuron_id }),
            command: Some(Command::Disburse(Disburse {
                to_account: None,
                amount: None
            }))
        })?;
        msgs.push(generate(pem, args).await?);
    };

    if msgs.is_empty() {
        return Err(anyhow!("No instructions provided"));
    }

    let mut out = String::new();
    out.push_str("[");
    out.push_str(&msgs.join(","));
    out.push_str("]");

    Ok(out)
}

pub async fn generate(pem: &Option<String>, args: Vec<u8>) -> AnyhowResult<String> {
    let method_name = "manage_neuron".to_string();
    let canister_id = Principal::from_text(GOVERNANCE_CANISTER_ID)?;
    let msg_with_req_id = sign(pem, canister_id.clone(), &method_name, args).await?;
    let request_id = msg_with_req_id
        .request_id
        .expect("No request id for transfer call found");
    let req_status_signed_msg = request_status::sign(pem, request_id, canister_id).await?;

    let mut out = String::new();
    out.push_str("{ \"ingress\": ");
    out.push_str(&msg_with_req_id.buffer);
    out.push_str(", \"request_status\": ");
    out.push_str(&req_status_signed_msg);
    out.push_str("}");

    Ok(out)
}
