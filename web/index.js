function connect() {
    return new Promise(function(resolve, reject) {
        // Create WebSocket connection.
        let socket = new WebSocket(`ws://${window.location.host}/log`);
        // Connection opened
        socket.addEventListener('open', function (event) {
            socket.send('Hello Server!');
            resolve(function(msg) {
                socket.send(msg);
            });
        });
        socket.addEventListener('error', function (event) {
            reject(event);
        });
    
        // Listen for messages
        socket.addEventListener('message', function (event) {
            console.log('Message from server ', event.data);
        });
    });
}
let log_div = document.getElementById("log");
function log_err(msg) {
    ws_log(msg);
    let p = document.createElement("p");
    p.appendChild(document.createTextNode(msg));
    log_div.appendChild(p);
}
var ws_log = function(msg) {
    console.log(msg);
}
async function init() {
    if (window.location.host !== "grafeia.github.io") {
        try {
            ws_log = await connect();
        } catch {
            log_err("can't connect logger");
        }
    }

    log_err("init");
    if (localStorage.getItem("reset")) {
        localStorage.clear();
    }
    log_err("ready for wasm");
    await wasm_bindgen("pkg/grafeia_web_bg.wasm").catch(function(e) {
        log_err(e);
    });
    log_err("wasm loaded");
    try {
        let app = new wasm_bindgen.Grafeia();
        app.show();
    } catch (e) {
        log_err(e);
    }
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
