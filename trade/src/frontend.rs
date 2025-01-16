use hypertext::*;

pub fn home() -> impl Renderable {
    rsx! {
        {Raw("<!DOCTYPE html>")}
        <html>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta name="darkreader-lock">
                //<link rel="icon" href="/static/img/favicon.svg">
                <title>Trade</title>
                <script src="https://cdn.tailwindcss.com/3.4.15"></script>
            </head>
            <body class="flex flex-col h-screen bg-white">
                <nav class="z-50 bg-white shadow-[0px_5px_10px_2px_rgba(0,0,0,0.3)]">
                    <div class="text-white bg-black">
                        <div class="flex justify-between mx-auto max-w-5xl">
                            <p class="my-auto pr-3">Trade</p>
                            <a href="#events" class="">Log Out</a>
                        </div>
                    </div>
                </nav>
                <main class="overflow-scroll text-black py-4 max-w-5xl w-full mx-auto">
                </main>
            </body>
        </html>
    }
}
