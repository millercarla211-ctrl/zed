Today is 11th April 2026, and we are working on zed code editor. I have attached an image screenshot, so study that and do this the current task is updating our web preview tab. Study the code about our web preview tab and implement these new updates: 
So in our web preview that we recently created, there is another code editor-like preview renderer in our Zed code editor. I want to do this: when we are in our web preview, it will move all of the current web preview tab bar to the bottom, like there is a secondary tab bar in the web preview. We don't want that so we are to put all of its buttons, input, and other elements at the top. When we are in web preview, we will not have any other code tabs open, right? It is straightforward. In the preview tab bar at the top right, there are currently three icon buttons:
1. +
2. Panel icon, third full screen icon. Currently, keep the class edition icon, and when it's only web preview, remove the other two icons. Instead of those icons, from the web preview tab bar, there is an extensions icon and some other icons. In the new web preview top bar, beside the ad icon, put an extensions icon. When we click on the extensions icon, please show a zy dropdown with all current listed extensions, and beside that please show a more icon. When we click on the more icon, show all of the current other stuff that we already have in our web preview secondary top bar.
The top left part will also be different than the other simple code editor view, as in those places, only in web preview, there are left arrow, right arrow, and then a retry or refresh icons. The new web preview main top bar will also have these three icons. Our previous web preview secondary top bar had an input box, but as the default Z code editor tabs there is a tab. We will still use that tab functionality, and we will only show an input box when we click on that tab. By default, it will still look like a tab, and just maybe for now, just keep it looking like a tab, and that's all. 
I get it that it's a pretty complex task, so before doing anything, you can ask me clarification questions so that we can implement this new web preview correctly. 

--------------------------------------------------------------------------------------------------------------------

Currently, in our web preview, we are showing the web preview using our hole punching system, which is pretty complex and really hard to debug. That's why we are going to put two ways to render our web preview.
Previously, I already added a right with preview method that shows the web preview, but it has a problem. We can show the GPU elements on top of it. In our web preview, create a modal in our top right, next to that add button. In there, by default use our Rai method and also put a selector to select the hole punching method too, so that we can choose which method we want to render our preview. By default, use the Rai method, which has this GPU element showing behind the problem.
Whenever we click on a native GPU element in our code editor, please make sure to:
- not render the web preview
- hide the web preview
- create a screenshot of that particular state of the web preview that previously was in
- hide the web preview
- put the image
- when the GPU element's actions have been over, show the web preview again
And don't ask me any clarification questions. Just try to do it autonomously as fast as possible!!!
