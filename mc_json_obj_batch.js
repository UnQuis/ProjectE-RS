
/*
 * Minecraft JSON -> OBJ Batch Exporter
 * Blockbench desktop plugin
 */

let batchJsonObjAction;

async function runBatch() {
    if (Blockbench.isWeb) {
        Blockbench.showMessageBox({
            title: 'Desktop app required',
            message: 'Batch conversion requires Blockbench desktop.'
        });
        return;
    }

    const root = Blockbench.pickDirectory({
        title: 'Select folder containing block/ and item/'
    });

    if (!root) return;

    const fs = requireNativeModule('fs', {
        scope: root,
        message: 'Allow reading models and writing OBJ exports.'
    });

    const path = requireNativeModule('path');

    if (!fs || !path) {
        Blockbench.showQuickMessage(
            'Filesystem permission denied.',
            3000
        );
        return;
    }

    const javaCodec = Codecs.java_block;
    const objCodec = Codecs.obj;

    if (
        !javaCodec ||
        typeof javaCodec.parse !== 'function' ||
        !objCodec ||
        typeof objCodec.compile !== 'function'
    ) {
        Blockbench.showMessageBox({
            title: 'Codec unavailable',
            message: 'Java Block/Item importer or OBJ exporter not found.'
        });
        return;
    }

    const outputRoot = path.join(root, 'obj_export');
    const files = [];

    // Recursive directory traversal
    function walk(dir) {
        for (const entry of fs.readdirSync(dir, {
            withFileTypes: true
        })) {
            const fullPath = path.join(dir, entry.name);

            if (entry.isDirectory()) {
                if (
                    path.resolve(fullPath) !==
                    path.resolve(outputRoot)
                ) {
                    walk(fullPath);
                }
            } else if (
                entry.isFile() &&
                entry.name.toLowerCase().endsWith('.json')
            ) {
                files.push(fullPath);
            }
        }
    }

    try {
        walk(root);
    } catch (error) {
        Blockbench.showMessageBox({
            title: 'Directory scan failed',
            message: String(error)
        });
        return;
    }

    if (!files.length) {
        Blockbench.showQuickMessage(
            'No JSON files found.',
            3000
        );
        return;
    }

    files.sort((a, b) => a.localeCompare(b));

    fs.mkdirSync(outputRoot, {
        recursive: true
    });

    let succeeded = 0;
    let failed = 0;

    const errors = [];

    function getOutputPath(inputPath) {
        const relative = path.relative(root, inputPath);

        return path.join(
            outputRoot,
            relative.replace(/\.json$/i, '.obj')
        );
    }

    Blockbench.showQuickMessage(
        `Starting conversion of ${files.length} models...`,
        2500
    );

    for (let i = 0; i < files.length; i++) {
        const inputPath = files[i];
        const outputPath = getOutputPath(inputPath);

        try {
            const raw = fs.readFileSync(
                inputPath,
                'utf8'
            );

            const json = JSON.parse(raw);

            // Fresh project for each model
            if (!newProject('java_block')) {
                throw new Error(
                    'Could not create Java Block project.'
                );
            }

            // Native Minecraft JSON importer
            javaCodec.parse(json, inputPath);

            // Native OBJ exporter
            const obj = objCodec.compile();

            if (
                typeof obj !== 'string' ||
                !obj.trim()
            ) {
                throw new Error(
                    'OBJ export returned empty output.'
                );
            }

            fs.mkdirSync(
                path.dirname(outputPath),
                         { recursive: true }
            );

            fs.writeFileSync(
                outputPath,
                obj,
                'utf8'
            );

            succeeded++;

        } catch (error) {
            failed++;

            errors.push({
                file: path.relative(root, inputPath),
                        message: String(
                            error?.message || error
                        )
            });
        }

        Blockbench.setProgress(
            (i + 1) / files.length
        );

        Blockbench.setStatusBarText(
            `JSON → OBJ: ${i + 1}/${files.length} | OK ${succeeded} | Failed ${failed}`
        );

        // Keep UI responsive
        if ((i + 1) % 5 === 0) {
            await new Promise(resolve =>
            setTimeout(resolve, 0)
            );
        }
    }

    Blockbench.setProgress(0);
    Blockbench.setStatusBarText();

    const report = [
        `Total: ${files.length}`,
        `Converted: ${succeeded}`,
        `Failed: ${failed}`,
        `Output: ${outputRoot}`
    ];

    if (errors.length) {
        const errorLog = errors.map(e =>
        `${e.file}\n  ${e.message}`
        ).join('\n\n');

        const logPath = path.join(
            outputRoot,
            '_conversion_errors.txt'
        );

        fs.writeFileSync(
            logPath,
            errorLog,
            'utf8'
        );

        report.push(`Error log: ${logPath}`);
    }

    Blockbench.showMessageBox({
        title: 'Batch conversion finished',
        message: report.join('\n')
    });
}

Plugin.register('mc_json_obj_batch', {
    title: 'Minecraft JSON → OBJ Batch Exporter',
    author: 'OpenAI',

    description:
    'Batch-convert Minecraft Java models to OBJ.',

    icon: 'fa-cubes',
    version: '1.0.0',
    variant: 'desktop',
    min_version: '5.0.0',

    onload() {
        batchJsonObjAction = new Action(
            'mc_json_obj_batch_run',
            {
                name: 'Batch Convert Minecraft JSON to OBJ',
                description:
                'Recursively convert JSON models to OBJ',
                icon: 'fa-cubes',
                click: runBatch
            }
        );

        MenuBar.menus.tools.addAction(
            batchJsonObjAction
        );
    },

    onunload() {
        if (batchJsonObjAction) {
            batchJsonObjAction.delete();
        }
    }
});
