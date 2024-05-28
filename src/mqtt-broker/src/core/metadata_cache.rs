use crate::metadata::{
    cluster::Cluster, connection::Connection, session::Session, subscriber::SubscribeData,
    topic::Topic, user::User,
};
use crate::{
    server::MQTTProtocol,
    storage::{cluster::ClusterStorage, topic::TopicStorage, user::UserStorage},
};
use common_base::log::warn;
use dashmap::DashMap;
use protocol::mqtt::{Subscribe, SubscribeProperties};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use storage_adapter::storage::StorageAdapter;
use tokio::time::sleep;

#[derive(Clone, Serialize, Deserialize)]
pub enum MetadataCacheAction {
    Set,
    Del,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum MetadataCacheType {
    Cluster,
    User,
    Topic,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetadataChangeData {
    pub action: MetadataCacheAction,
    pub data_type: MetadataCacheType,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetadataCacheManager {
    // cluster_name
    pub cluster_name: String,

    // (cluster_name, Cluster)
    pub cluster_info: DashMap<String, Cluster>,

    // (username, User)
    pub user_info: DashMap<String, User>,

    // (client_id, Session)
    pub session_info: DashMap<String, Session>,

    // (connect_id, Connection)
    pub connection_info: DashMap<u64, Connection>,

    // (topic_name, Topic)
    pub topic_info: DashMap<String, Topic>,

    // (topic_id, topic_name)
    pub topic_id_name: DashMap<String, String>,

    // (client_id, <pkid,SubscribeData>)
    pub subscribe_filter: DashMap<String, DashMap<u16, SubscribeData>>,

    // (client_id, vec<pkid>)
    pub publish_pkid_info: DashMap<String, Vec<u16>>,
}

impl MetadataCacheManager {
    pub fn new(cluster_name: String) -> Self {
        let cache = MetadataCacheManager {
            cluster_name,
            cluster_info: DashMap::with_capacity(1),
            user_info: DashMap::with_capacity(256),
            session_info: DashMap::with_capacity(256),
            topic_info: DashMap::with_capacity(256),
            topic_id_name: DashMap::with_capacity(256),
            connection_info: DashMap::with_capacity(256),
            subscribe_filter: DashMap::with_capacity(8),
            publish_pkid_info: DashMap::with_capacity(8),
        };
        return cache;
    }

    pub fn add_client_subscribe(
        &self,
        client_id: String,
        protocol: MQTTProtocol,
        subscribe: Subscribe,
        subscribe_properties: Option<SubscribeProperties>,
    ) {
        if let Some(data) = self.subscribe_filter.get_mut(&client_id) {
            data.insert(
                subscribe.packet_identifier,
                SubscribeData {
                    protocol,
                    subscribe,
                    subscribe_properties,
                },
            );
        } else {
            let data = DashMap::with_capacity(8);
            data.insert(
                subscribe.packet_identifier,
                SubscribeData {
                    protocol,
                    subscribe,
                    subscribe_properties,
                },
            );
            self.subscribe_filter.insert(client_id, data);
        };
    }

    pub fn remove_filter(&self, client_id: String) {
        self.subscribe_filter.remove(&client_id);
    }

    pub fn apply(&self, data: String) {
        let data: MetadataChangeData = serde_json::from_str(&data).unwrap();
        match data.data_type {
            MetadataCacheType::User => match data.action {
                MetadataCacheAction::Set => {
                    let user: User = serde_json::from_str(&data.value).unwrap();
                    self.add_user(user);
                }
                MetadataCacheAction::Del => self.del_user(data.value),
            },
            MetadataCacheType::Topic => match data.action {
                MetadataCacheAction::Set => {}
                MetadataCacheAction::Del => {}
            },
            MetadataCacheType::Cluster => match data.action {
                MetadataCacheAction::Set => {}
                MetadataCacheAction::Del => {}
            },
        }
    }

    pub fn set_cluster_info(&self, cluster: Cluster) {
        self.cluster_info.insert(self.cluster_name.clone(), cluster);
    }

    pub fn get_cluster_info(&self) -> Cluster {
        return self.cluster_info.get(&self.cluster_name).unwrap().clone();
    }

    pub fn add_user(&self, user: User) {
        self.user_info.insert(user.username.clone(), user);
    }

    pub fn del_user(&self, value: String) {
        let data: User = serde_json::from_str(&value).unwrap();
        self.user_info.remove(&data.username);
    }

    pub fn add_session(&self, client_id: String, session: Session) {
        self.session_info.insert(client_id, session);
    }

    pub fn add_connection(&self, connect_id: u64, conn: Connection) {
        self.connection_info.insert(connect_id, conn);
    }

    pub fn add_topic(&self, topic_name: &String, topic: &Topic) {
        let t = topic.clone();
        self.topic_info.insert(topic_name.clone(), t.clone());
        self.topic_id_name.insert(t.topic_id, topic_name.clone());
    }

    pub fn login_success(&self, connect_id: u64) {
        if let Some(mut conn) = self.connection_info.get_mut(&connect_id) {
            conn.login = true;
        }
    }

    pub fn is_login(&self, connect_id: u64) -> bool {
        if let Some(mut conn) = self.connection_info.get_mut(&connect_id) {
            return conn.login;
        }
        return false;
    }

    pub fn topic_exists(&self, topic: &String) -> bool {
        return self.topic_info.contains_key(topic);
    }

    pub fn topic_name_by_id(&self, topic_id: String) -> Option<String> {
        if let Some(data) = self.topic_id_name.get(&topic_id) {
            return Some(data.clone());
        }
        return None;
    }

    pub fn get_topic_by_name(&self, topic_name: String) -> Option<Topic> {
        if let Some(topic) = self.topic_info.get(&topic_name) {
            return Some(topic.clone());
        }
        return None;
    }

    pub fn remove_connection(&self, connect_id: u64, client_id: String) {
        self.session_info.remove(&client_id);
        self.connection_info.remove(&connect_id);
        self.subscribe_filter.remove(&client_id);
        self.publish_pkid_info.remove(&client_id);
    }

    pub fn get_topic_alias(&self, connect_id: u64, topic_alias: u16) -> Option<String> {
        if let Some(conn) = self.connection_info.get_mut(&connect_id) {
            if let Some(topic_name) = conn.topic_alias.get(&topic_alias) {
                return Some(topic_name.clone());
            } else {
                return None;
            }
        }
        return None;
    }

    pub fn get_connect_id(&self, client_id: String) -> Option<u64> {
        if let Some(sess) = self.session_info.get(&client_id) {
            if let Some(conn_id) = sess.connection_id {
                return Some(conn_id);
            }
        }
        return None;
    }

    pub async fn get_pkid(&self, client_id: String) -> u16 {
        let pkid = self.get_available_pkid(client_id.clone()).await;
        if let Some(mut pkid_list) = self.publish_pkid_info.get_mut(&client_id) {
            pkid_list.push(pkid);
        } else {
            self.publish_pkid_info.insert(client_id, vec![pkid]);
        }
        return pkid;
    }

    async fn get_available_pkid(&self, client_id: String) -> u16 {
        loop {
            if let Some(pkid_list) = self.publish_pkid_info.get(&client_id) {
                for i in 0..65535 {
                    if pkid_list.contains(&i) {
                        continue;
                    }
                    return i;
                }
            } else {
                return 0;
            }
            sleep(Duration::from_millis(10)).await;
            warn("No pkid available for client, wait 10ms.".to_string());
        }
    }

    pub fn remove_pkid_info(&self, client_id: String, pkid: u16) {
        if let Some(mut pkid_list) = self.publish_pkid_info.get_mut(&client_id) {
            pkid_list.retain(|x| *x == pkid);
        }
    }

    pub fn client_pkid_size(&self, client_id: String, pkid: u16) -> usize {
        if let Some(mut pkid_list) = self.publish_pkid_info.get_mut(&client_id) {
            return pkid_list.len();
        }
        return 0;
    }
}

pub async fn load_metadata_cache<T>(
    metadata_storage_adapter: Arc<T>,
) -> (Cluster, DashMap<String, User>, DashMap<String, Topic>)
where
    T: StorageAdapter + Sync + Send + 'static + Clone,
{
    // load cluster config
    let cluster_storage = ClusterStorage::new(metadata_storage_adapter.clone());
    let cluster = match cluster_storage.get_cluster_config().await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => Cluster::new(),
        Err(e) => {
            panic!(
                "Failed to load the cluster configuration with error message:{}",
                e.to_string()
            );
        }
    };

    // load all user
    let user_storage = UserStorage::new(metadata_storage_adapter.clone());
    let user_list = match user_storage.user_list().await {
        Ok(list) => list,
        Err(e) => {
            panic!(
                "Failed to load the user list with error message:{}",
                e.to_string()
            );
        }
    };

    let user_info = DashMap::with_capacity(8);
    for (username, user) in user_list {
        user_info.insert(username, user);
    }

    // load topic info
    let topic_storage = TopicStorage::new(metadata_storage_adapter.clone());
    let topic_list = match topic_storage.topic_list().await {
        Ok(list) => list,
        Err(e) => {
            panic!(
                "Failed to load the topic list with error message:{}",
                e.to_string()
            );
        }
    };

    let topic_info = DashMap::with_capacity(8);

    for (topic_name, topic) in topic_list {
        topic_info.insert(topic_name.clone(), topic.clone());
    }
    return (cluster, user_info, topic_info);
}
