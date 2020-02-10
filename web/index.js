function connect() {
    return new Promise(function(resolve, reject) {
        // Create WebSocket connection.
        const socket = new WebSocket(`ws://${window.location.host}/log`);
        // Connection opened
        socket.addEventListener('open', function (event) {
            socket.send('Hello Server!');
            resolve(function(msg) {
                socket.send(msg);
            });
        });
    
        // Listen for messages
        socket.addEventListener('message', function (event) {
            console.log('Message from server ', event.data);
        });
    });
}

let ws_log;
async function init() {
    ws_log = await connect();

    ws_log("init");
    if (localStorage.getItem("reset")) {
        localStorage.clear();
    }
    ws_log("ready for wasm");
    await wasm_bindgen("pkg/grafeia_web_bg.wasm").catch(function(e) {
        console.error(e);
        document.getElementById("status").innerText += " failed.";
    });
    ws_log("wasm loaded");
    let app = new wasm_bindgen.Grafeia();
    app.show();
}
init();

function test_unload() {
    var event = document.createEvent('Event');
    event.initEvent('unload', true, true);
    window.dispatchEvent(event);
}
function reset() {
    localStorage.setItem("reset", "reset");
    window.location.reload();
}
