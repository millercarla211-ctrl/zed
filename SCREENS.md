Today is 12th April 2026, and we are working on zed code editor. I have attached an image screenshot, so study that and do this the current task is updating our web preview tab. Study the code about our web preview tab and implement these new updates:

In our current gen co-editor, there is a status, and in the top bar, at the most top bar, there is space in the center. In there, please create a rounded pill rectangle like a macOS dock. In there, please put two items on the left side:
1. Path
2. The branch, our current working branch (that our j code editor already does in the top bar, main top bar left side, but we should move that to our Mac ways screen dock)
We will put that dock in the top bar center. On the left side, as we discuss, we will put two items that are locked. In the center of our Mac ways screen dock, we will put, by default, three macOS light icons:
1. Editor
2. Browser
3. Terminal
On the right side of our Mac ways screen dock, we will put one ad icon and one list icon. As I have shown, all the items in our Mac ways screen dock in the top bar here are the things that all the items will do. Let's start with the path and branch - It's pretty simple, and it's exactly what it should be in the current top bar on the left side. Currently, we already are showing path and branch details. We don't show that there anymore. We will show it in our mac ways screen dock at the center of our top bar, and then in the center of our mac ways trend there will be icons and maybe levels that I have listed. In the end, clicking them will make the current screen the click item. If I click on the code icon, it will make the code the default current center panel of our zcode editor, and in the case when we will click browser or terminal as the feature, it will make the browser and terminal the default ones.

On the right side of our macOS dock, there is an add and list icon right in there. When we click on the add icon, it will show us a dropdown or select items using our ZGPUIComponent select component. It will list files and all other strings that zcode editor has, and it will then put that with a dedicated icon and increase the icons besides the three default ones that we already have added.

On the right side of our add button in our macOS trend off there will be a list icon, and when we click on the list icon it will show us all the screens with some details in a GPUI popover, so it will look nice. The main thing of our this string system is that currently, in our code editor, in the secondary top bar, we are trying to show browser, terminal, and code in one place. From now on, we will only show the things in one string. Here is the thing: when we are on code editor, clicking on the add button from now on will only create a new file and will not show a dropdown of creating multiple types of different things. When we are in a code editor, we don't need to create a browser preview next to it, but when we are clicking on the browser, it will only create a new browser preview. It will make the design of our zcode editor more professional and efficient, so that's why, from our zcode editor main centered code panel view or the tabs main centered tabs, please create dedicated streams instead of putting all the streams beside it.

Now, here's the thing: we will put things together but with the same category. When we are on the browser tab, browser screen, we will only create a new web browser preview when we click on the add icon. It doesn't need to create another terminal icon in the same place. If you need a terminal, then you go to the terminal screen and create one. That's how we will make these coder editor screens more functional.

Here's the thing: as currently all the screens will have a dedicated category, like way and system, that's why we can make a carousel out of it like this. For the first, there will be the Code editor tab string. Right now, when we are hovering on the right and left edge of our string, it will outline or highlight that screen to be like a draggable one. When we are dragging and resizing the string, then the other strings are like beside each other, like a carousel. If we resize a string to be smaller, it will make the next strings, in this case the browser string, come into the view, because the browser screen will stay there. If we make the code screen smaller from the right side, it will make the browser screen come into the view, and that goes for the left side resize too. When we resize the code screen from the left side, then the last screen should appear from the left side. That's the logic. When we are resizing it and other screens in our view, when we click on this screen, it will make the screen the current screen, so that it will make it a good user experience. In this way, we will be able to have a nice screen here, like a UI, so that a user can better use all the tabs correctly. It is a really ground-breaking, out-of-the-box, game-changing way of developer experience. 

I get it that it's a pretty complex task, so before doing anything, you can ask me clarification questions so that we can implement this new web preview correctly. 








































Awesome now please make our screen dock small in height and decrease the icon size so that it can fit in the small height. Currently the border radius on our screen dock is so much, like maybe full border radius, but only put 10px border radius.

Currently here is the main issue: when we are clicking on the browser preview, it is putting the browser preview next to the code editor tab but the code editor tab is completely different from the browser, right? That is why if we are showing the browser tab, make sure no code editor tab should be present on the tab. They will be removed or just create another instance of the browser tab without any kind of code editing tab or code editor related tab. On the code editor there should only be code related tabs, no other stuff.

Currently when we are clicking on the terminal, it is creating a bottom terminal view. Please make sure the browser that has a dedicated full screen will also have a full screen and in there too only the terminal will show; nothing else will show. This is the main logic of our screen dock. Make sure to put the border screen resizing capabilities and the ability to resize other screens into view. When we click on the parts of the resizing other screen, it will make that screen the default with full width and every screen will maintain its different kinds of width so that even though one screen is smaller than the whole width as the user edits the screen to be small, when we are on another screen it will by default get the full width. Only if the user also customizes the width of the screen will it show the effect.

Make sure to put proper ruling so that when it is the fast screen, as for our case the currently code editor tab is the first screen. When we resize from the left side it will resize the terminal view into the terminal screen into the view, as the terminal is the third screen, the third and last screen. The last screen will come from the left side. When we resize the first screen, the code editor screen, from the right side, the next second screen that is the browser screen will come into view. When we click on the parts of the browser string it will make the default screen the browser screen and highlight the dock correctly. Even though the first string has a smaller width, it is supposed to be on the left side as a hanging tab. As it is the browser screen that hasn't been customized in width, it will by default get the full width. When we add in the third string, when we resize from the right side, it will show the first code editor screen as it will have a carousel-like effect. When we resize from the left side in our third terminal screen, it will make the browser screen in the view. Make sure that we can add as many screens as we like correctly.

Please make the screen dock in the top bar center, even though the left side gets expanded, but still it should be in the center correctly and not push the left menu item. Please fix that, and also please make all the text icons inside the screen dock smaller and put a 5 px border radius. You are right about the browser and terminal screen, but they are currently blank, so here is the thing: the pod editor Screen can already render browser screen and terminal screen, so the problem you are facing is that you have to create a new layout to render browser and terminal. That's not even the problem. User spams code editor screen in those two places, and just make sure that in those two places only their specific stuff get created, not others. That's how easy it is. 
