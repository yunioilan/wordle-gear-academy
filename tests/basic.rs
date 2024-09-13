use game_session_io::*;
use gtest::{Log, ProgramBuilder, System};

const GAME_SESSION_PROGRAM_ID: u64 = 1;
const WORDLE_PROGRAM_ID: u64 = 2;
const USER: u64 = 50; // 学号为 50

#[test]
fn test_win() {
    let system = System::new();
    system.init_logger();

    // 部署 game_session 和 wordle 程序
    let game_session_program = ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
        .with_id(GAME_SESSION_PROGRAM_ID)
        .build(&system);
    let wordle_program = ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
        .with_id(WORDLE_PROGRAM_ID)
        .build(&system);

    // 初始化 Wordle 程序
    let res = wordle_program.send_bytes(USER, []);
    assert!(!res.main_failed());

    // 初始化 GameSession 程序
    let res = game_session_program.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!res.main_failed());

    // 尝试在没有开始游戏的情况下检查单词（应该失败）
    let res = game_session_program.send(
        USER,
        GameSessionAction::CheckWord {
            word: "abcde".to_string(),
        },
    );
    assert!(res.main_failed());

    // 开始游戏
    let res = game_session_program.send(USER, GameSessionAction::StartGame);
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::StartSuccess);
    assert!(!res.main_failed() && res.contains(&log));

    // 再次尝试开始游戏（应该失败，因为游戏已经开始）
    let res = game_session_program.send(USER, GameSessionAction::StartGame);
    assert!(res.main_failed());

    // 尝试输入无效单词（不符合规则，应该失败）
    let res = game_session_program.send(
        USER,
        GameSessionAction::CheckWord {
            word: "Abcde".to_string(),
        },
    );
    assert!(res.main_failed());

    let res = game_session_program.send(
        USER,
        GameSessionAction::CheckWord {
            word: "abcdef".to_string(),
        },
    );
    assert!(res.main_failed());

    // 输入合法单词并验证结果
    let res = game_session_program.send(
        USER,
        GameSessionAction::CheckWord {
            word: "house".to_string(),
        },
    );
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::CheckWordResult {
            correct_positions: vec![0, 1, 3, 4],
            contained_in_word: vec![],
        });
    assert!(!res.main_failed() && res.contains(&log));

    // 输入正确单词并获胜
    let res = game_session_program.send(
        USER,
        GameSessionAction::CheckWord {
            word: "horse".to_string(),
        },
    );
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::GameOver(GameStatus::Win));
    assert!(!res.main_failed() && res.contains(&log));

    // 另一个用户尝试检查单词（没有开始游戏，应该失败）
    let res = game_session_program.send(
        51,
        GameSessionAction::CheckWord {
            word: "abcde".to_string(),
        },
    );
    assert!(res.main_failed());

    // 输出当前游戏状态
    let state: GameSessionState = game_session_program.read_state(b"").unwrap();
    println!("{:?}", state);
}

#[test]
fn test_tried_limit() {
    let system = System::new();
    system.init_logger();

    // 部署 game_session 和 wordle 程序
    let game_session_program = ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
        .with_id(GAME_SESSION_PROGRAM_ID)
        .build(&system);
    let wordle_program = ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
        .with_id(WORDLE_PROGRAM_ID)
        .build(&system);

    // 初始化 Wordle 程序
    let res = wordle_program.send_bytes(USER, []);
    assert!(!res.main_failed());

    // 初始化 GameSession 程序
    let res = game_session_program.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!res.main_failed());

    // 开始游戏
    let res = game_session_program.send(USER, GameSessionAction::StartGame);
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::StartSuccess);
    assert!(!res.main_failed() && res.contains(&log));

    // 测试猜测次数限制
    for i in 0..5 {
        let res = game_session_program.send(
            USER,
            GameSessionAction::CheckWord {
                word: "house".to_string(),
            },
        );
        if i == 4 {
            let log = Log::builder()
                .dest(USER)
                .source(GAME_SESSION_PROGRAM_ID)
                .payload(GameSessionEvent::GameOver(GameStatus::Lose));
            assert!(!res.main_failed() && res.contains(&log));
        } else {
            let log = Log::builder()
                .dest(USER)
                .source(GAME_SESSION_PROGRAM_ID)
                .payload(GameSessionEvent::CheckWordResult {
                    correct_positions: vec![0, 1, 3, 4],
                    contained_in_word: vec![],
                });
            assert!(!res.main_failed() && res.contains(&log));
        }
    }
    // 输出当前游戏状态
    let state: GameSessionState = game_session_program.read_state(b"").unwrap();
    println!("{:?}", state);
}

#[test]
#[ignore] // 延迟逻辑的执行
fn test_delayed_logic() {
    let system = System::new();
    system.init_logger();

    // 部署 game_session 和 wordle 程序
    let game_session_program = ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
        .with_id(GAME_SESSION_PROGRAM_ID)
        .build(&system);
    let wordle_program = ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
        .with_id(WORDLE_PROGRAM_ID)
        .build(&system);

    // 初始化 Wordle 程序
    let res = wordle_program.send_bytes(USER, []);
    assert!(!res.main_failed());

    // 初始化 GameSession 程序
    let res = game_session_program.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!res.main_failed());

    // 开始游戏
    let res = game_session_program.send(USER, GameSessionAction::StartGame);
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::StartSuccess);
    assert!(!res.main_failed() && res.contains(&log));

    // 模拟等待200个区块的延迟
    let result = system.spend_blocks(200);
    println!("{:?}", result);

    // 检查游戏是否超时并失败
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::GameOver(GameStatus::Lose));
    assert!(result[0].contains(&log));

    // 输出当前游戏状态
    let state: GameSessionState = game_session_program.read_state(b"").unwrap();
    println!("{:?}", state);
}
