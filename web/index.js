async function init() {
    let r = await fetch("pkg/web_bg.wasm");
    let data = await r.arrayBuffer();
    let module = await wasm_bindgen(data);
    console.log(module);
}
init();