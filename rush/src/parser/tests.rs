use crate::lexer::Lexer;
use crate::types::*;

use super::parse;

fn parse_input(input: &str) -> Program {
    let lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    parse(&tokens).unwrap()
}

fn first_command(program: &Program) -> &Command {
    &program.items[0].list.first.commands[0]
}

// ── simple commands ──────────────────────────────────────────

#[test]
fn simple_command_bare() {
    let p = parse_input("echo hello");
    assert_eq!(p.items.len(), 1);
    let cmd = first_command(&p);
    assert_eq!(cmd.name.as_str(), "echo");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].as_str(), "hello");
}

#[test]
fn pipeline_two_commands() {
    let p = parse_input("echo hello | cat");
    assert_eq!(p.items.len(), 1);
    let pipe = &p.items[0].list.first;
    assert_eq!(pipe.commands.len(), 2);
    assert_eq!(pipe.commands[0].name.as_str(), "echo");
    assert_eq!(pipe.commands[1].name.as_str(), "cat");
}

#[test]
fn semicolon_separated() {
    let p = parse_input("echo a; echo b");
    assert_eq!(p.items.len(), 2);
    assert_eq!(p.items[0].list.first.commands[0].name.as_str(), "echo");
    assert_eq!(p.items[1].list.first.commands[0].name.as_str(), "echo");
}

#[test]
fn newline_separated() {
    let p = parse_input("echo a\necho b\n");
    assert_eq!(p.items.len(), 2);
}

#[test]
fn background_command() {
    let p = parse_input("sleep 10 &");
    assert!(p.items[0].background);
}

// ── and-or lists ──────────────────────────────────────────────

#[test]
fn and_list() {
    let p = parse_input("gcc -c foo.c && gcc -c bar.c");
    assert_eq!(p.items.len(), 1);
    let list = &p.items[0].list;
    assert_eq!(list.first.commands[0].name.as_str(), "gcc");
    assert_eq!(list.rest.len(), 1);
}

#[test]
fn or_list() {
    let p = parse_input("cmd1 || cmd2");
    let list = &p.items[0].list;
    assert_eq!(list.rest.len(), 1);
}

// ── compound commands ────────────────────────────────────────

#[test]
fn subshell() {
    let p = parse_input("(echo hello)");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::Subshell(inner) => {
            assert_eq!(inner.items.len(), 1);
        }
        _ => panic!("expected subshell"),
    }
}

#[test]
fn if_clause_simple() {
    let p = parse_input("if true; then echo yes; fi");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::If(ifc) => {
            assert_eq!(ifc.condition.items.len(), 1);
            assert_eq!(ifc.body.items.len(), 1);
            assert!(ifc.elifs.is_empty());
            assert!(ifc.else_body.is_none());
        }
        _ => panic!("expected if"),
    }
}

#[test]
fn if_clause_with_else() {
    let p = parse_input("if false; then echo no; else echo yes; fi");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::If(ifc) => {
            assert!(ifc.else_body.is_some());
        }
        _ => panic!("expected if"),
    }
}

#[test]
fn if_clause_with_elif() {
    let p = parse_input("if false; then echo no; elif true; then echo yes; fi");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::If(ifc) => {
            assert_eq!(ifc.elifs.len(), 1);
        }
        _ => panic!("expected if"),
    }
}

#[test]
fn while_loop() {
    let p = parse_input("while true; do echo hi; done");
    let cmd = first_command(&p);
    matches!(&cmd.kind, CommandKind::While(_));
}

#[test]
fn for_loop() {
    let p = parse_input("for f in a b c; do echo $f; done");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::For(fc) => {
            assert_eq!(fc.name.as_str(), "f");
            assert_eq!(fc.words.len(), 3);
        }
        _ => panic!("expected for"),
    }
}

#[test]
fn for_loop_multiline() {
    let p = parse_input("for f in a b c; do\necho $f\ndone");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::For(fc) => {
            assert_eq!(fc.name.as_str(), "f");
            assert_eq!(fc.words.len(), 3);
            assert_eq!(fc.body.items.len(), 1);
        }
        _ => panic!("expected for, got {:?}", cmd.kind),
    }
}

#[test]
fn case_single_arm() {
    let p = parse_input("case x in a) echo hi;; esac");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::Case(cc) => {
            assert_eq!(cc.arms.len(), 1);
            assert_eq!(cc.arms[0].patterns.len(), 1);
            assert_eq!(cc.arms[0].patterns[0].as_str(), "a");
        }
        _ => panic!("expected case, got {:?}", cmd.kind),
    }
}

#[test]
fn case_two_arms() {
    let p = parse_input("case $x in a) echo one;; b) echo two;; esac");
    let cmd = first_command(&p);
    match &cmd.kind {
        CommandKind::Case(cc) => {
            assert_eq!(cc.arms.len(), 2);
            assert_eq!(cc.arms[0].patterns.len(), 1);
        }
        _ => panic!("expected case"),
    }
}

#[test]
fn pipeline_with_negation() {
    let p = parse_input("! grep foo");
    assert!(p.items[0].list.first.negation);
}

#[test]
fn redirect_in_simple_command() {
    let p = parse_input("echo hello > out.txt");
    let cmd = first_command(&p);
    assert_eq!(cmd.redirects.len(), 1);
    assert_eq!(cmd.redirects[0].target.as_str(), "out.txt");
}

#[test]
fn multiple_commands_with_newlines() {
    let p = parse_input("echo a\nif true; then echo b; fi\necho c");
    assert_eq!(p.items.len(), 3);
}
