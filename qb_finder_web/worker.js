importScripts("pkg/qb_finder_web.js");

async function main() {
    let legal_boards;

    let response = await fetch(
        "https://wirelyre.github.io/tetra-tools/legal-boards.leb128",
    );
    if (response.ok) {
        legal_boards = new Uint8Array(await response.arrayBuffer());
    } else {
        console.log("couldn't load legal boards");
    }

    await wasm_bindgen("pkg/qb_finder_web_bg.wasm");
    let qbf = new wasm_bindgen.QBF(legal_boards);
    postMessage({ kind: "ready" });

    onmessage = (msg) => {
        let query = msg.data;

        if (query.setup != undefined) {
            postMessage({
                kind: "ok",
                res: qbf.find_min_sets(
                    query.setup,
                    query.build_queue,
                    query.solve_queue,
                    query.save,
                ),
            });
            return;
        }

        qbf.set_skip_4p(query.skip_4p);

        let setups = qbf.find(
            query.build_queue.toUpperCase(),
            query.solve_queue.toUpperCase(),
            query.save.toUpperCase(),
        )
            .split("|");

        if (setups[0] == "") {
            setups = [];
        }

        postMessage({ kind: "ok", query, setups: setups });
    };
}

main();
