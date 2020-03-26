let ws_callback = null;
let ws = null;
let log_div = document.getElementById("log");
const ERROR = 1;
const WARN = 2;
const INFO = 3;
const DEBUG = 4;
const TRACE = 5;
const log_fns = {
    1: s => console.error(s),
    2: s => console.warn(s),
    3: s => console.info(s),
    4: s => console.debug(s),
    5: s => console.trace(s),
}
function log(level, msg) {
    log_fns[level](msg);
    if (ws) {
        ws.send(msg);
    }
    if (level <= INFO) {
        let p = document.createElement("p");
        p.appendChild(document.createTextNode(msg));
        log_div.appendChild(p);
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
function data_url() {
    let location = window.location;
    let name = "demo";
    if (location.hash) {
        name = location.hash.substr(1);
    }
    return `${location.protocol}//${location.host}${location.pathname}${name}.graf`;
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
let spin_fan = false;
let view;
async function init_view(socket) {
    let canvas = document.getElementById("canvas");
    let capture = document.getElementById("capture");
    if (socket) {
        view = wasm_bindgen.online(canvas);
    } else {
        let url = data_url();
        log(INFO, `fetching document from ${url}`);
        let response = await fetch(url);
        if (response.status != 200) {
            log(ERROR, `${url} not found`);
            return;
        }
        let data = new Uint8Array(await response.arrayBuffer());

        log(INFO, "initializing");
        view = wasm_bindgen.offline(canvas, data);
    }

    let requested = false;
    function animation_frame(time) {
        requested = false;
        let t0 = performance.now();
        view.animation_frame(time);
        let t1 = performance.now();
        let dt = t1 - t0;
        console.log(`${dt}ms (${1000 / dt}fps)`);

        if (spin_fan) {
            request_animation_frame();
        }
    }

    function request_animation_frame() {
        if (!requested) {
            window.requestAnimationFrame(animation_frame);
            requested = true;
        }
    }

    function check(_request_redraw) {
        let request_redraw = view.idle();
        if (request_redraw) {
            request_animation_frame();
        }
    }

    window.addEventListener("keydown", e => check(view.key_down(e)), {capture: true});
    window.addEventListener("keyup", e => check(view.key_up(e)), {capture: true});
    capture.addEventListener("mousemove", e => check(view.mouse_move(e)));
    capture.addEventListener("mouseup", e => check(view.mouse_up(e)));
    capture.addEventListener("mousedown", e => check(view.mouse_down(e)));
    window.addEventListener("resize", e => check(view.resize(e)));
    window.addEventListener("paste", e => check(view.paste(e)));
    capture.addEventListener("input", e => check(view.input(e.data)));
    ws_callback = data => check(view.data(data));
}

async function init() {
    log(3, "ready for wasm");
    await wasm_bindgen("pkg/grafeia_web_bg.wasm").catch(function(e) {
        log(1, e);
    });
    log(3, "wasm loaded");

    if (window.location.host !== "grafeia.github.io") {
        try {
            ws = await connect();
        } catch {
            log(2, "can't connect logger");
        }
    }

    try {
        await init_view(ws);
        view.idle();
        view.render();
    } catch (e) {
        log(1, e);
    }
}
init();
