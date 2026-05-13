use super::{
    ExecutedActionReceipt, FlowCommandExecution, FlowPermissionGate, FlowTextExecution,
    contracts::FlowAutomationBridge,
    engine::FlowEngine,
    session::FlowSessionContext,
    types::{AppContext, TypingAssistRequest},
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowSelectionExecution {
    pub original_selection: String,
    pub text_execution: FlowTextExecution,
    pub replaced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowShortcutExecution {
    pub shortcut: String,
    pub executed: bool,
    pub receipt: ExecutedActionReceipt,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowAutomationEngine;

impl FlowAutomationEngine {
    pub fn rewrite_selection<P, A>(
        &self,
        engine: &FlowEngine,
        context: &mut FlowSessionContext,
        permissions: &mut P,
        bridge: &mut A,
    ) -> Option<FlowSelectionExecution>
    where
        P: FlowPermissionGate,
        A: FlowAutomationBridge,
    {
        let selection = bridge.read_selection()?;
        let app_context = AppContext::default();
        let execution = engine.process_text(
            context,
            TypingAssistRequest {
                text: selection.clone(),
                app_context: app_context.clone(),
                dictionary: engine.session.hub.dictionary_for_context(),
                snippets: engine.session.hub.snippets_for_context(),
                styles: engine.session.hub.styles_for_context(&app_context),
                auto_correct: true,
                expand_snippets: true,
            },
            permissions,
            &mut super::contracts::RecordingControlExecutor::default(),
        );
        let replaced = if let Some(ref receipt) = execution.insert_receipt {
            if receipt.executed {
                bridge.replace_selection(&execution.pass.typing.final_text)
            } else {
                false
            }
        } else {
            false
        };

        Some(FlowSelectionExecution {
            original_selection: selection,
            text_execution: execution,
            replaced,
        })
    }

    pub fn dispatch_shortcut<A>(
        &self,
        bridge: &mut A,
        shortcut: impl Into<String>,
    ) -> FlowShortcutExecution
    where
        A: FlowAutomationBridge,
    {
        let shortcut = shortcut.into();
        let executed = bridge.send_shortcut(&shortcut);
        FlowShortcutExecution {
            receipt: ExecutedActionReceipt {
                capability: super::control::ControlCapability::SimulateShortcut,
                executed,
                message: if executed {
                    "Shortcut dispatched through the automation bridge.".to_string()
                } else {
                    "Shortcut dispatch failed through the automation bridge.".to_string()
                },
            },
            shortcut,
            executed,
        }
    }

    pub fn command_shortcut<A>(
        &self,
        bridge: &mut A,
        command: &FlowCommandExecution,
    ) -> Option<FlowShortcutExecution>
    where
        A: FlowAutomationBridge,
    {
        if matches!(
            command.pass.command.intent,
            super::command::FlowCommandIntent::FocusOverlay
        ) {
            return Some(self.dispatch_shortcut(bridge, "Alt+`"));
        }

        None
    }
}
