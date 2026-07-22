use std::fs;
use std::path::Path;

use file_vault::cli_helpers::{
	ensure_runtime_directories,
	parse_runtime_mode,
	runtime_directories,
	RuntimeMode,
};

#[test]
fn test_parse_runtime_mode() {
	let (runtime_mode, args) = parse_runtime_mode(vec![
		"--test".to_string(),
		"localhost".to_string(),
		"8080".to_string(),
	]);

	assert_eq!(RuntimeMode::Test, runtime_mode);
	assert_eq!(vec!["localhost".to_string(), "8080".to_string()], args);
}

#[test]
fn test_ensure_runtime_directories_in_test_mode() {
	let (log_directory, server_directory) = runtime_directories(RuntimeMode::Test);

	let _ = fs::remove_dir_all(&log_directory);
	let _ = fs::remove_dir_all(&server_directory);

	assert!(ensure_runtime_directories(RuntimeMode::Test).is_ok());
	assert!(Path::new(&log_directory).exists());
	assert!(Path::new(&server_directory).exists());

	let _ = fs::remove_dir_all(&log_directory);
	let _ = fs::remove_dir_all(&server_directory);
}

#[test]
fn test_ensure_runtime_directories_in_production_mode() {
	let (log_directory, server_directory) = runtime_directories(RuntimeMode::Production);

	let _ = fs::remove_dir_all(&log_directory);
	let _ = fs::remove_dir_all(&server_directory);

	assert!(ensure_runtime_directories(RuntimeMode::Production).is_ok());
	assert!(Path::new(&log_directory).exists());
	assert!(Path::new(&server_directory).exists());

	let _ = fs::remove_dir_all(&log_directory);
	let _ = fs::remove_dir_all(&server_directory);
}