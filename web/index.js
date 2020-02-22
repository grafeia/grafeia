let ws_callback = null;
let ws = null;
function ws_log(msg) {
    console.log(msg);
    if (ws) {
        ws.send(msg);
    }
};
function ws_send(data) {
    if (ws) {
        ws.send(data);
    }
}
function set_scroll_factors(factors) {
    // pixel delta factor x and y
    factors[0] = 0.4;
    factors[1] = 0.4;

    // line delta factor x and y
    factors[2] = 30.;
    factors[3] = 10.0;
}
function set_ws_callback(cb) {
    ws_callback = cb;
}
function blob2ArrayBuffer(blob) {
    if (blob.arrayBuffer) {
        return blob.arrayBuffer();
    }
    return new Promise(function(resolve, reject) {
        let reader = new FileReader();
        reader.onload = function() {
            resolve(reader.result);
        };
        reader.readAsArrayBuffer(blob);
    });
}
function ws_url() {
    let location = window.location;
    let protocol = { "http:": "ws:", "https:": "wss:" }[location.protocol];
    return `${protocol}//${window.location.host}/log`;
}
function connect() {
    return new Promise(function(resolve, reject) {
        // Create WebSocket connection.
        let socket = new WebSocket(ws_url());
        // Connection opened
        socket.addEventListener('open', function (event) {
            socket.send('Hello Server!');
            resolve(socket);
        });
        socket.addEventListener('error', function (event) {
            reject(event);
            ws = null;
        });
        // Listen for messages
        socket.addEventListener('message', function (event) {
            console.log('Message from server ', event.data);
            blob2ArrayBuffer(event.data).then(function(data) {
                ws_callback(new Uint8Array(data));
            });
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
            ws = await connect();
        } catch {
            log_err("can't connect logger");
        }
    }
    log_err("ready for wasm");
    await wasm_bindgen("pkg/grafeia_web_bg.wasm").catch(function(e) {
        log_err(e);
    });
    log_err("wasm loaded");
    try {
        if (ws) {
            wasm_bindgen.online();
        } else {
            wasm_bindgen.offline();
        }
    } catch (e) {
        log_err(e);
    }
}
init();
