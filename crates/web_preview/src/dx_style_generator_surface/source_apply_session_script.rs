const DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS_SCRIPT: &str = r##"    const sourceApplySessionKind = sourceApplyContract.source_apply_session_kind || "unknown";
    const sourceApplySessionToken = __DX_STYLE_SOURCE_APPLY_SESSION_TOKEN__;
    const sourceApplyMaxSessionTokenBytes = Number(sourceApplyContract.max_source_apply_session_token_bytes || 0);
"##;

const DX_STYLE_SOURCE_APPLY_SESSION_HANDLER_SCRIPT: &str = r##"    function sourceApplyHandler() {
      const handler = typeof window.__DX_STYLE_SOURCE_APPLY__ === "function"
        ? window.__DX_STYLE_SOURCE_APPLY__
        : null;
      return handler?.can_mutate_source === true ? handler : null;
    }

    function sourceApplyReviewHandler() {
      const handler = typeof window.__DX_STYLE_SOURCE_APPLY__ === "function"
        ? window.__DX_STYLE_SOURCE_APPLY__
        : null;
      return handler?.can_review_request === true ? handler : null;
    }

    function sourceApplyHandlerState() {
      const handler = typeof window.__DX_STYLE_SOURCE_APPLY__ === "function"
        ? window.__DX_STYLE_SOURCE_APPLY__
        : null;
      if (!handler) return "not_enabled";
      if (handler.can_mutate_source === true) return "ready";
      if (handler.can_review_request === true) return "review_only";
      return "unknown";
    }

    function installSourceApplyHandler() {
      if (typeof window.__DX_STYLE_SOURCE_APPLY__ === "function") return;
      if (typeof window.ipc?.postMessage !== "function") return;
      const handler = (request) => {
        window.ipc.postMessage(JSON.stringify({
          kind: sourceApplyIpcKind,
          request,
          source_apply_session: {
            kind: sourceApplySessionKind,
            token: sourceApplySessionToken
          },
          handler_capability: {
            can_review_request: true,
            can_mutate_source: false
          }
        }));
      };
      Object.defineProperty(handler, "can_review_request", { value: true });
      Object.defineProperty(handler, "can_mutate_source", { value: false });
      Object.defineProperty(window, "__DX_STYLE_SOURCE_APPLY__", { value: handler });
    }
"##;

pub(super) fn dx_style_source_apply_session_constants_script() -> &'static str {
    DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS_SCRIPT
}

pub(super) fn dx_style_source_apply_session_handler_script() -> &'static str {
    DX_STYLE_SOURCE_APPLY_SESSION_HANDLER_SCRIPT
}
