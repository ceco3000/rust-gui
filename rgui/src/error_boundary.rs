//! 组件异常隔离——panic 恢复与错误回退。
//!
//! 实现 D1 §11.3 的组件异常处理策略：
//! - `view()` panic → 渲染错误占位符
//! - `update()` panic → 回滚状态（clone 前值）
//! - `paint()` panic → 跳过该 widget，渲染错误占位符
//!
//! 使用 `std::panic::catch_unwind` 捕获用户代码 panic，
//! 确保单个组件异常不影响其他组件的渲染和交互。

use rgui_core::traits::AppMessage;
use rgui_core::view::WidgetView;
use std::panic::{AssertUnwindSafe, catch_unwind, set_hook, take_hook};

/// 安装自定义 panic hook，防止 `catch_unwind` 触发 `SIGABRT`。
///
/// 应在应用启动时调用一次。替换默认 hook 为静默版本，
/// 这样 `catch_unwind` 捕获的 panic 不会导致进程退出。
pub fn install_panic_hook() {
    let prev = take_hook();
    set_hook(Box::new(move |info| {
        // 将 panic 信息写入 stderr，但不触发默认行为（abort）
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        let location = info
            .location()
            .map_or_else(String::new, |l| format!(" at {}:{}", l.file(), l.line()));
        log::error!(target: "rgui::core", "[rgui] 组件异常: {msg}{location}");
        // 仍调用原有 hook 以保留双输出行为（可选）
        prev(info);
    }));
}

/// 安全执行 view 闭包。
///
/// 如果闭包 panic，返回错误占位符 WidgetView。
pub fn catch_view<M: AppMessage>(f: impl FnOnce() -> WidgetView<M>) -> WidgetView<M> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(view) => view,
        Err(_) => error_placeholder_view(),
    }
}

/// 安全执行 update 闭包，支持状态回滚。
///
/// 调用前 clone 状态，如果闭包 panic 则恢复 clone。
/// 如果闭包成功执行，状态保持修改后的值。
pub fn catch_update<S: Clone>(state: &mut S, f: impl FnOnce(&mut S)) {
    let snapshot = state.clone();
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: AssertUnwindSafe wrapper; state 通过闭包引用传递
        let s = unsafe { &mut *(state as *mut S) };
        f(s);
    }));
    if result.is_err() {
        // 回滚到 panic 前的状态
        *state = snapshot;
    }
}

/// Paint 异常结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintError;

impl std::fmt::Display for PaintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "paint operation panicked")
    }
}

/// 安全执行 paint 闭包。
///
/// 返回 `Ok(())` 表示正常执行，`Err(PaintError)` 表示 panic。
pub fn catch_paint(f: impl FnOnce()) -> Result<(), PaintError> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| PaintError)
}

/// 创建错误占位符 WidgetView。
///
/// 当组件 `view()` panic 时，框架使用此占位符渲染红底白叉的矩形区域。
/// widget_type 固定为 `"__rgui_error_placeholder__"`，内部组件不应使用此名称。
#[must_use]
pub fn error_placeholder_view<M: AppMessage>() -> WidgetView<M> {
    WidgetView::new("__rgui_error_placeholder__")
        .prop("width", 200.0_f64)
        .prop("height", 200.0_f64)
        .prop("background_color", "rgba(255,0,0,0.3)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::traits::AppMessage;
    use rgui_core::view::PropValue;

    #[derive(Debug, Clone, PartialEq)]
    struct TestMessage;
    impl AppMessage for TestMessage {
        fn message_name(&self) -> &'static str {
            "TestMessage"
        }
    }

    // ========================================================================
    // catch_view 测试
    // ========================================================================

    #[test]
    fn catch_view_returns_normal_view_when_no_panic() {
        let view = catch_view(|| WidgetView::<TestMessage>::new("TestWidget"));
        assert_eq!(view.widget_type, "TestWidget");
    }

    #[test]
    fn catch_view_returns_error_placeholder_on_panic() {
        let view: WidgetView<TestMessage> = catch_view(|| {
            panic!("view 崩溃");
        });
        assert_eq!(view.widget_type, "__rgui_error_placeholder__");
        // 错误占位符有默认尺寸属性
        let w = view.props.get("width");
        assert!(matches!(w, Some(PropValue::Float(f)) if f.0 == 200.0));
    }

    #[test]
    fn catch_view_preserves_view_when_successful() {
        let view = catch_view(|| {
            WidgetView::<TestMessage>::new("Button")
                .prop("label", "Click me")
                .prop("width", 100.0_f64)
        });
        assert_eq!(view.widget_type, "Button");
        let label = view.props.get("label");
        assert!(matches!(label, Some(PropValue::Str(s)) if s.as_ref() == "Click me"));
    }

    // ========================================================================
    // catch_update 测试
    // ========================================================================

    #[test]
    fn catch_update_applies_state_when_successful() {
        let mut state = 0_i32;
        catch_update(&mut state, |s| {
            *s = 42;
        });
        assert_eq!(state, 42);
    }

    #[test]
    fn catch_update_rolls_back_on_panic() {
        let mut state = 100_i32;
        catch_update(&mut state, |s| {
            *s = 200;
            panic!("update 崩溃");
        });
        // 状态应回滚到 100
        assert_eq!(state, 100);
    }

    #[test]
    fn catch_update_works_with_complex_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct Counter {
            value: i32,
            label: String,
        }

        let mut state = Counter {
            value: 0,
            label: "计数".into(),
        };

        catch_update(&mut state, |s| {
            s.value += 1;
            s.label = format!("计数: {}", s.value);
        });
        assert_eq!(state.value, 1);
        assert_eq!(state.label, "计数: 1");
    }

    #[test]
    fn catch_update_rolls_back_complex_type_on_panic() {
        #[derive(Clone, Debug, PartialEq)]
        struct Config {
            url: String,
            timeout: u32,
        }

        let mut state = Config {
            url: "http://example.com".into(),
            timeout: 30,
        };

        catch_update(&mut state, |s| {
            s.url = "http://modified.com".into();
            s.timeout = 60;
            panic!("配置更新失败");
        });

        assert_eq!(state.url, "http://example.com");
        assert_eq!(state.timeout, 30);
    }

    // ========================================================================
    // catch_paint 测试
    // ========================================================================

    #[test]
    fn catch_paint_returns_ok_when_no_panic() {
        let result = catch_paint(|| {
            let _x = 1 + 1;
        });
        assert!(result.is_ok());
    }

    #[test]
    fn catch_paint_returns_err_on_panic() {
        let result = catch_paint(|| {
            panic!("paint 崩溃");
        });
        assert!(result.is_err());
    }

    // ========================================================================
    // error_placeholder_view 测试
    // ========================================================================

    #[test]
    fn error_placeholder_has_correct_widget_type() {
        let view = error_placeholder_view::<TestMessage>();
        assert_eq!(view.widget_type, "__rgui_error_placeholder__");
    }

    #[test]
    fn error_placeholder_has_default_dimensions() {
        let view = error_placeholder_view::<TestMessage>();
        assert!(matches!(
            view.props.get("width"),
            Some(PropValue::Float(f)) if f.0 == 200.0
        ));
        assert!(matches!(
            view.props.get("height"),
            Some(PropValue::Float(f)) if f.0 == 200.0
        ));
    }

    #[test]
    fn error_placeholder_has_background() {
        let view = error_placeholder_view::<TestMessage>();
        let bg = view.props.get("background_color");
        assert!(matches!(bg, Some(PropValue::Str(s)) if s.as_ref() == "rgba(255,0,0,0.3)"));
    }

    // ========================================================================
    // catch_view with catch_update integration
    // ========================================================================

    #[test]
    fn view_and_update_isolation_does_not_interfere() {
        // view 和 update 的异常隔离互不影响
        let mut state = 0_i32;

        // view 正常
        let _view = catch_view(|| WidgetView::<TestMessage>::new("Test"));
        assert_eq!(state, 0);

        // update 正常
        catch_update(&mut state, |s| *s = 10);
        assert_eq!(state, 10);

        // view panic
        let _view2: WidgetView<TestMessage> = catch_view(|| {
            panic!("view error");
        });
        // state 不变
        assert_eq!(state, 10);
    }
}
