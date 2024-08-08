#![no_std]
use game_session_io::*;
use gstd::{exec, msg};

// 尝试次数的上限
const TRIES_LIMIT: u8 = 5;

// 全局静态变量，用于存储游戏会话状态
static mut GAME_SESSION_STATE: Option<GameSession> = None;

#[no_mangle]
extern "C" fn init() {
        let game_session_init: GameSessionInit = msg::load().expect("无法解码 GameSessionInit");
        game_session_init.assert_valid(); // 验证数据有效性
    unsafe {
        // 初始化游戏会话状态
        GAME_SESSION_STATE = Some(game_session_init.into());
    };
}

#[no_mangle]
extern "C" fn handle() {
    // 解码并处理游戏会话动作
    let game_session_action: GameSessionAction = msg::load().expect("无法解码 GameSessionAction");
    let game_session = get_game_session_mut();
    match game_session_action {
        GameSessionAction::StartGame => {
            let user = msg::source(); // 获取消息发送者，即玩家
            let session_info = game_session.sessions.entry(user).or_default();
            match &session_info.session_status {
                SessionStatus::ReplyReceived(wordle_event) => {
                    // 如果之前收到过回复，则回复玩家游戏已启动
                    msg::reply::<GameSessionEvent>(wordle_event.into(), 0).expect("回复消息失败");
                    session_info.session_status = SessionStatus::WaitUserInput; // 更新状态为等待玩家输入
                }
                SessionStatus::Init
                | SessionStatus::GameOver(..)
                | SessionStatus::WaitWordleStartReply => {
                    // 向Wordle程序发送"StartGame"消息
                    let send_to_wordle_msg_id = msg::send(
                        game_session.wordle_program_id,
                        WordleAction::StartGame { user },
                        0,
                    ).expect("发送消息失败");
                    session_info.session_id = msg::id(); // 保存当前消息ID
                    session_info.original_msg_id = msg::id(); // 保存初始消息ID
                    session_info.send_to_wordle_msg_id = send_to_wordle_msg_id; // 保存发送到Wordle的消息ID
                    session_info.tries = 0; // 初始化尝试次数
                    session_info.session_status = SessionStatus::WaitWordleStartReply; // 更新状态为等待Wordle启动回复

                    msg::send_delayed(
                        exec::program_id(),
                        GameSessionAction::CheckGameStatus {
                            user,
                            session_id: msg::id(),
                        },
                        1000000,
                        200,
                    ).expect("发送延迟消息失败");
                    exec::wait();  // 等待回复
                }
                SessionStatus::WaitUserInput | SessionStatus::WaitWordleCheckWordReply => {
                    panic!("用户已经在游戏中");
                }
            }
        }
        GameSessionAction::CheckWord { word } => {
            let user = msg::source();
            let session_info = game_session.sessions.entry(user).or_default();
            match &session_info.session_status {
                SessionStatus::ReplyReceived(wordle_event) => {
                    session_info.tries += 1; // 增加尝试次数
                    if wordle_event.has_guessed() {
                        // 如果猜对了单词，游戏结束并设置状态为胜利
                        session_info.session_status = SessionStatus::GameOver(GameStatus::Win);
                        msg::reply(GameSessionEvent::GameOver(GameStatus::Win), 0)
                            .expect("回复消息失败");
                    } else if session_info.tries == TRIES_LIMIT {
                        // 如果达到尝试次数限制，游戏结束并设置状态为失败
                        session_info.session_status = SessionStatus::GameOver(GameStatus::Lose);
                        msg::reply(GameSessionEvent::GameOver(GameStatus::Lose), 0).expect("回复消息失败");
                    } else {
                        msg::reply::<GameSessionEvent>(wordle_event.into(), 0)
                            .expect("回复消息失败");
                        session_info.session_status = SessionStatus::WaitUserInput;
                        // 更新状态为等待玩家输入
                    }
                }
                SessionStatus::WaitUserInput | SessionStatus::WaitWordleCheckWordReply => {
                    // 验证提交的单词长度是否为五，并且所有字母为小写
                    assert!(
                        word.len() == 5 && word.chars().all(|c| c.is_lowercase()),
                        "无效的单词"
                    );
                    let send_to_wordle_msg_id = msg::send(
                        game_session.wordle_program_id,
                        WordleAction::CheckWord { user, word },
                        0,
                    ).expect("发送消息失败");
                    session_info.original_msg_id = msg::id();
                    session_info.send_to_wordle_msg_id = send_to_wordle_msg_id;
                    session_info.session_status = SessionStatus::WaitWordleCheckWordReply; // 更新状态为等待Wordle检查单词回复
                    exec::wait(); // 等待回复
                }
                SessionStatus::Init
                | SessionStatus::WaitWordleStartReply
                | SessionStatus::GameOver(..) => {
                    panic!("用户不在游戏中");
                }
            }
        }
        GameSessionAction::CheckGameStatus { user, session_id } => {
            if msg::source() == exec::program_id() {
                if let Some(session_info) = game_session.sessions.get_mut(&user) {
                    if session_id == session_info.session_id
                        && !matches!(session_info.session_status, SessionStatus::GameOver(..))
                    {
                        session_info.session_status = SessionStatus::GameOver(GameStatus::Lose); // 如果时间到未完成，游戏结束并设置状态为失败
                        msg::send(user, GameSessionEvent::GameOver(GameStatus::Lose), 0).expect("发送消息失败");
                    }
                }
            }
        }
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    let reply_to = msg::reply_to().expect("查询 reply_to 数据失败");
    let wordle_event: WordleEvent = msg::load().expect("无法解码 WordleEvent");
    let game_session = get_game_session_mut();
    let user = wordle_event.get_user();
    if let Some(session_info) = game_session.sessions.get_mut(user) {
        if reply_to == session_info.send_to_wordle_msg_id && session_info.is_wait_reply_status() {
            session_info.session_status = SessionStatus::ReplyReceived(wordle_event); // 收到Wordle程序的回复
            exec::wake(session_info.original_msg_id).expect("唤醒消息失败");
        }
    }
}

#[no_mangle]
extern "C" fn state() {
    let game_session = get_game_session();
    msg::reply::<GameSessionState>(game_session.into(), 0).expect("状态查询回复失败");
}


fn get_game_session_mut() -> &'static mut GameSession {
    unsafe { GAME_SESSION_STATE.as_mut().expect("游戏会话未初始化") }
}
fn get_game_session() -> &'static GameSession {
    unsafe {
        GAME_SESSION_STATE.as_ref().expect("游戏会话未初始化") }
}