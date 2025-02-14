use std::fmt;
use std::path::Path;

use base64::Engine;
use tracing::{error, info};

use flowy_server_config::af_cloud_config::AFCloudConfiguration;
use flowy_server_config::supabase_config::SupabaseConfiguration;
use flowy_user::services::entities::URL_SAFE_ENGINE;
use lib_infra::file_util::copy_dir_recursive;

use crate::integrate::log::create_log_filter;

#[derive(Clone)]
pub struct AppFlowyCoreConfig {
  /// Different `AppFlowyCoreConfig` instance should have different name
  pub(crate) name: String,
  pub(crate) device_id: String,
  /// Used to store the user data
  pub storage_path: String,
  /// Origin application path is the path of the application binary. By default, the
  /// storage_path is the same as the origin_application_path. However, when the user
  /// choose a custom path for the user data, the storage_path will be different from
  /// the origin_application_path.
  pub application_path: String,
  pub(crate) log_filter: String,
  cloud_config: Option<AFCloudConfiguration>,
}

impl fmt::Debug for AppFlowyCoreConfig {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut debug = f.debug_struct("AppFlowy Configuration");
    debug.field("storage_path", &self.storage_path);
    debug.field("application_path", &self.application_path);
    if let Some(config) = &self.cloud_config {
      debug.field("base_url", &config.base_url);
      debug.field("ws_url", &config.ws_base_url);
      debug.field("gotrue_url", &config.gotrue_url);
    }
    debug.finish()
  }
}

fn migrate_local_version_data_folder(root: &str, url: &str) -> String {
  // Isolate the user data folder by using the base url of AppFlowy cloud. This is to avoid
  // the user data folder being shared by different AppFlowy cloud.
  let storage_path = if !url.is_empty() {
    let server_base64 = URL_SAFE_ENGINE.encode(url);
    format!("{}_{}", root, server_base64)
  } else {
    root.to_string()
  };

  // Copy the user data folder from the root path to the isolated path
  // The root path without any suffix is the created by the local version AppFlowy
  if !Path::new(&storage_path).exists() && Path::new(root).exists() {
    info!("Copy dir from {} to {}", root, storage_path);
    let src = Path::new(root);
    match copy_dir_recursive(src, Path::new(&storage_path)) {
      Ok(_) => storage_path,
      Err(err) => {
        // when the copy dir failed, use the root path as the storage path
        error!("Copy dir failed: {}", err);
        root.to_string()
      },
    }
  } else {
    storage_path
  }
}

impl AppFlowyCoreConfig {
  pub fn new(
    custom_application_path: String,
    application_path: String,
    device_id: String,
    name: String,
  ) -> Self {
    let cloud_config = AFCloudConfiguration::from_env().ok();
    let storage_path = match &cloud_config {
      None => {
        let supabase_config = SupabaseConfiguration::from_env().ok();
        match &supabase_config {
          None => custom_application_path,
          Some(config) => migrate_local_version_data_folder(&custom_application_path, &config.url),
        }
      },
      Some(config) => migrate_local_version_data_folder(&custom_application_path, &config.base_url),
    };

    AppFlowyCoreConfig {
      name,
      storage_path,
      application_path,
      device_id,
      log_filter: create_log_filter("info".to_owned(), vec![]),
      cloud_config,
    }
  }

  pub fn log_filter(mut self, level: &str, with_crates: Vec<String>) -> Self {
    self.log_filter = create_log_filter(level.to_owned(), with_crates);
    self
  }
}
