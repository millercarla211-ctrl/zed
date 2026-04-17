Today is 7th April 2026, and this is the Zed-Code Editor. Now do me a favor and explain to me how the sidebar works. Under the sidebar, there are some items. Explain to me the source code of the sidebar for the switch sidebar.

Now do research about the latest Codex CLI with GPT 5.4 model, as today is 7th February 2026, and update the agents.md file to tell our Codex agent to not run any other command related to building; instead, use just the run command, and let our agent know about our current webview implementations and the new features or web preview correctly so that the agent can work on it from now on.

This is the real hole-puncing: make sure you don't have to do anything. Codex will implement it; you just tell codex to do it correctly. And also tell codex to format and lint all files!!!
The main problem with standard webviews is that they float *on top* of the app, blocking UI menus (like Zed's Command Palette or tooltips). 
With Hole-Punching (Underlay), you put the native OS webview **BEHIND** the GPUI window. You make the GPUI window background transparent. 
*   If a webpage is just sitting there, it shows through the transparent GPUI window.
*   If you open Zed's Command Palette, GPUI draws it normally. Because GPUI is the top layer, the Command Palette perfectly overlaps the webview. 
*The only limitation:* A webview's internal dropdown menu cannot float *over* a GPUI element. But 99% of the time, you want the editor's UI to have priority over the webview anyway.
