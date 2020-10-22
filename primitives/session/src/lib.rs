// This file is part of Substrate.

// Copyright (C) 2019-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Substrate core types around sessions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};

#[cfg(feature = "std")]
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
#[cfg(feature = "std")]
use sp_api::ProvideRuntimeApi;

use sp_core::RuntimeDebug;
use sp_core::crypto::KeyTypeId;
use sp_runtime::traits::Convert;
use sp_runtime::{RuntimeAppPublic, BoundToRuntimeAppPublic};
use sp_staking::SessionIndex;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	/// Session keys runtime api.
	pub trait SessionKeys {
		/// Generate a set of session keys with optionally using the given seed.
		/// The keys should be stored within the keystore exposed via runtime
		/// externalities.
		///
		/// The seed needs to be a valid `utf8` string.
		///
		/// Returns the concatenated SCALE encoded public keys.
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8>;

		/// Decode the given public session keys.
		///
		/// Returns the list of public raw public keys + key type.
		fn decode_session_keys(encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, KeyTypeId)>>;
	}
}

/// Number of validators in a given session.
pub type ValidatorCount = u32;

/// Proof of membership of a specific key in a given session.
#[derive(Encode, Decode, Clone, Eq, PartialEq, Default, RuntimeDebug)]
pub struct MembershipProof {
	/// The session index on which the specific key is a member.
	pub session: SessionIndex,
	/// Trie nodes of a merkle proof of session membership.
	pub trie_nodes: Vec<Vec<u8>>,
	/// The validator count of the session on which the specific key is a member.
	pub validator_count: ValidatorCount,
}

/// A utility trait to get a session number. This is implemented for
/// `MembershipProof` below to fetch the session number the given session
/// membership proof is for. It is useful when we need to deal with key owner
/// proofs generically (i.e. just typing against the `KeyOwnerProofSystem`
/// trait) but still restrict their capabilities.
pub trait GetSessionNumber {
	fn session(&self) -> SessionIndex;
}

/// A utility trait to get the validator count of a given session. This is
/// implemented for `MembershipProof` below and fetches the number of validators
/// in the session the membership proof is for. It is useful when we need to
/// deal with key owner proofs generically (i.e. just typing against the
/// `KeyOwnerProofSystem` trait) but still restrict their capabilities.
pub trait GetValidatorCount {
	fn validator_count(&self) -> ValidatorCount;
}

impl GetSessionNumber for sp_core::Void {
	fn session(&self) -> SessionIndex {
		Default::default()
	}
}

impl GetValidatorCount for sp_core::Void {
	fn validator_count(&self) -> ValidatorCount {
		Default::default()
	}
}

impl GetSessionNumber for MembershipProof {
	fn session(&self) -> SessionIndex {
		self.session
	}
}

impl GetValidatorCount for MembershipProof {
	fn validator_count(&self) -> ValidatorCount {
		self.validator_count
	}
}

/// Trait for retrieving the session info needed for online node inspection.
///
/// This trait is used for decouple the pallet-session dependency from im-online
/// module so that the user of im-online & offences modules can pass any list of
/// validators that are considered to be online in each session, particularly useful
/// for the Substrate-based projects having their own staking implementation
/// instead of using pallet-staking directly.
pub trait ValidatorSet<AccountId> {
	// TODO [ToDr] This could use `frame_support::Parameter` instead,although don't know if such
	// import is legal.
	type ValidatorId: codec::Codec + codec::EncodeLike + Clone + Eq + sp_std::fmt::Debug;
	// TODO [ToDr] This is most likely not needed along with `AccountId`
	type ValidatorIdOf: Convert<AccountId, Option<Self::ValidatorId>>;

	/// Returns current session index.
	fn current_index() -> SessionIndex;

	/// Returns all the validators ought to be online in a session.
	///
	/// The returned validators are all expected to be running an authority node.
	fn validators() -> Vec<Self::ValidatorId>;
}

/// `ValidatorSet` combined with identification type for pallet-session-historical module.
pub trait ValidatorSetWithIdentification<AccountId>: ValidatorSet<AccountId> {
	type Identification: codec::Codec + codec::EncodeLike + Clone + Eq + sp_std::fmt::Debug;
	type IdentificationOf: Convert<Self::ValidatorId, Option<Self::Identification>>;
}

/// A session handler for specific key type.
pub trait OneSessionHandler<ValidatorId>: BoundToRuntimeAppPublic {
	/// The key type expected.
	type Key: Decode + Default + RuntimeAppPublic;

	fn on_genesis_session<'a, I: 'a>(validators: I)
		where I: Iterator<Item=(&'a ValidatorId, Self::Key)>, ValidatorId: 'a;

	/// Session set has changed; act appropriately. Note that this can be called
	/// before initialization of your module.
	///
	/// `changed` is true when at least one of the session keys
	/// or the underlying economic identities/distribution behind one the
	/// session keys has changed, false otherwise.
	///
	/// The `validators` are the validators of the incoming session, and `queued_validators`
	/// will follow.
	fn on_new_session<'a, I: 'a>(
		changed: bool,
		validators: I,
		queued_validators: I,
	) where I: Iterator<Item=(&'a ValidatorId, Self::Key)>, ValidatorId: 'a;


	/// A notification for end of the session.
	///
	/// Note it is triggered before any `SessionManager::end_session` handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {}

	/// A validator got disabled. Act accordingly until a new session begins.
	fn on_disabled(_validator_index: usize);
}

/// Generate the initial session keys with the given seeds, at the given block and store them in
/// the client's keystore.
#[cfg(feature = "std")]
pub fn generate_initial_session_keys<Block, T>(
	client: std::sync::Arc<T>,
	at: &BlockId<Block>,
	seeds: Vec<String>,
) -> Result<(), sp_api::ApiErrorFor<T, Block>>
where
	Block: BlockT,
	T: ProvideRuntimeApi<Block>,
	T::Api: SessionKeys<Block>,
{
	let runtime_api = client.runtime_api();

	for seed in seeds {
		runtime_api.generate_session_keys(at, Some(seed.as_bytes().to_vec()))?;
	}

	Ok(())
}
