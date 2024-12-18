// Copyright 2023 RobustMQ Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use rocksdb_engine::engine::{rocksdb_engine_get, rocksdb_engine_save};
use rocksdb_engine::RocksDBEngine;

use super::keys::{offset_segment_end, offset_segment_position, offset_segment_start};
use crate::core::consts::DB_COLUMN_FAMILY_INDEX;
use crate::core::error::JournalServerError;
use crate::segment::SegmentIdentity;

pub struct OffsetIndexManager {
    rocksdb_engine_handler: Arc<RocksDBEngine>,
}

impl OffsetIndexManager {
    pub fn new(rocksdb_engine_handler: Arc<RocksDBEngine>) -> Self {
        OffsetIndexManager {
            rocksdb_engine_handler,
        }
    }

    pub fn save_start_offset(
        &self,
        segment_iden: &SegmentIdentity,
        start_offset: u64,
    ) -> Result<(), JournalServerError> {
        let key = offset_segment_start(segment_iden);
        Ok(rocksdb_engine_save(
            self.rocksdb_engine_handler.clone(),
            DB_COLUMN_FAMILY_INDEX,
            key,
            start_offset,
        )?)
    }

    pub fn get_start_offset(
        &self,
        segment_iden: &SegmentIdentity,
    ) -> Result<u64, JournalServerError> {
        let key = offset_segment_start(segment_iden);
        if let Some(res) = rocksdb_engine_get(
            self.rocksdb_engine_handler.clone(),
            DB_COLUMN_FAMILY_INDEX,
            key,
        )? {
            return Ok(serde_json::from_slice::<u64>(&res.data)?);
        }

        Ok(0)
    }

    pub fn save_end_offset(
        &self,
        segment_iden: &SegmentIdentity,
        end_offset: u64,
    ) -> Result<(), JournalServerError> {
        let key = offset_segment_end(segment_iden);
        Ok(rocksdb_engine_save(
            self.rocksdb_engine_handler.clone(),
            DB_COLUMN_FAMILY_INDEX,
            key,
            end_offset,
        )?)
    }

    pub fn get_end_offset(
        &self,
        segment_iden: &SegmentIdentity,
    ) -> Result<u64, JournalServerError> {
        let key = offset_segment_end(segment_iden);
        if let Some(res) = rocksdb_engine_get(
            self.rocksdb_engine_handler.clone(),
            DB_COLUMN_FAMILY_INDEX,
            key,
        )? {
            return Ok(serde_json::from_slice::<u64>(&res.data)?);
        }

        Ok(0)
    }

    pub fn save_position_offset(
        &self,
        segment_iden: &SegmentIdentity,
        offset: u64,
        position: u64,
    ) -> Result<(), JournalServerError> {
        let key = offset_segment_position(segment_iden, offset);
        Ok(rocksdb_engine_save(
            self.rocksdb_engine_handler.clone(),
            DB_COLUMN_FAMILY_INDEX,
            key,
            position,
        )?)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn offset_index_test() {}
}
