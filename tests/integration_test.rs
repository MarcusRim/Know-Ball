#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

// Test that the program starts and shows welcome message
#[test]
fn test_program_starts() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("quit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Welcome to Know Ball"));
}

// Test that list command shows available questions
#[test]
fn test_list_command() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("list\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available question codes:"))
        .stdout(predicate::str::contains("last10passers_TEAM"));
}

// Test that quit command exits gracefully
#[test]
fn test_quit_command() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("quit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Goodbye!"));
}

// Test exit command also works
#[test]
fn test_exit_command() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Goodbye!"));
}

// Test invalid command shows error message
#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("notacommand\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Unknown command or code"));
}

// Test that a valid team-specific question is recognized
#[test]
fn test_valid_team_question() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    // Type the command then immediately reveal to end the trivia
    cmd.write_stdin("last10passers_PIT\nreveal\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Team: PIT"))
        .stdout(predicate::str::contains("TRIVIA"));
}

// Test that start command generates a random question
#[test]
fn test_start_command() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("start\nreveal\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Random code:"))
        .stdout(predicate::str::contains("TRIVIA"));
}

// Test case insensitivity for commands
#[test]
fn test_case_insensitive_commands() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("LIST\nQUIT\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available question codes:"));
}

// Test invalid team code
#[test]
fn test_invalid_team_code() {
    let mut cmd = Command::cargo_bin("know_ball").unwrap();

    cmd.write_stdin("last10passers_XYZ\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Unknown command or code"));
}
