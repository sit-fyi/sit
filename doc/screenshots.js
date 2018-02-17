// It's a very hastily written script just to serve a very specific purpose
// If you care, please improve upon it.
var path = require('path'),
    doc = path.dirname(__filename); 
    child_process = require('child_process'),
    puppeteer = require('puppeteer'),
    tmpdir = require('unique-temp-dir'),
    process = require('process'),
    which = require('which');

function launch() {
        // @yrashk: I run NixOS, so bundled chrome is not going to cut it for
        // me (not going to start at all)
        return puppeteer.launch({executablePath: which.sync("chromium")});
}

function available(web) {
        return new Promise((resolve) => {
                web.stdout.on('data', (data) => {
                        if (data.toString().startsWith("Serving")) {
                                setTimeout(() => resolve(true), 100);
                        }
                });
        });
}

async function empty_screen() {
        var temp = tmpdir({create: true});
        child_process.execSync("sit init", {cwd: temp});
        var web = child_process.spawn("sit-web", ["127.0.0.1:10801"], {cwd: temp});

        await available(web);

        const browser = await launch();
        const page = await browser.newPage();
        page.setCacheEnabled(false);
        await page.goto('http://localhost:10801/', {waitUntil: 'networkidle0'});
        const screenshot = await page.screenshot({path: "getting_started/empty_screen.png"});
        web.kill();
        return browser.close();
}

async function new_issue() {
        var temp = tmpdir({create: true});
        child_process.execSync("sit init", {cwd: temp});
        var web = child_process.spawn("sit-web", ["127.0.0.1:10802"], {cwd: temp});

        await available(web);

        const browser = await launch();
        const page = await browser.newPage();
        page.setCacheEnabled(false);
        await page.goto('http://localhost:10802/new', {waitUntil: 'networkidle0'});
        // new issue form
        const screenshot = await page.screenshot({path: "getting_started/new_issue.png"});
        // newly created issue
        const summary = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-new').shadowRoot.querySelector('paper-input[id="summary"]')`);
        const summaryE = summary.asElement();
        await summaryE.focus();
        await page.keyboard.type("Learning SIT should be easy");
        const details = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-new').shadowRoot.querySelector('paper-textarea[id="details"]')`);
        const detailsE = details.asElement();
        await detailsE.focus();
        await page.keyboard.type("All materials, demos, etc. should be easily accessible");
        const create = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-new').shadowRoot.querySelector('paper-button')`);
        const createE = create.asElement();
        await createE.click();
        await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view')`);
        const screenshot2 = await page.screenshot({path: "getting_started/new_created_issue.png"});
        // commenting
        const comment = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view').shadowRoot.querySelector('issue-new-comment').shadowRoot.querySelector('paper-textarea')`);
        const commentE = comment.asElement();
        await commentE.focus();
        await page.keyboard.type("Indeed, it is extremely important!");
        const button = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view').shadowRoot.querySelector('issue-new-comment').shadowRoot.querySelector('paper-button')`);
        const buttonE = button.asElement();
        await buttonE.click();
        await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view').shadowRoot.querySelector('issue-comment')`);
 
        const screenshot3 = await page.screenshot({path: "getting_started/new_comment.png"});
        // issues
        await page.goto('http://localhost:10802/', {waitUntil: 'networkidle0'});
        const screenshot4 = await page.screenshot({path: "getting_started/issues.png"});
        web.kill();
        return browser.close();
}

async function search() {
        var temp = tmpdir({create: true});
        child_process.execSync("sit init", {cwd: temp});
        var web = child_process.spawn("sit-web", ["127.0.0.1:10803"], {cwd: temp});

        await available(web);

        const browser = await launch();
        const page = await browser.newPage();
        page.setCacheEnabled(false);
        await page.goto('http://localhost:10803/new', {waitUntil: 'networkidle0'});
        const summary = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-new').shadowRoot.querySelector('paper-input[id="summary"]')`);
        const summaryE = summary.asElement();
        await summaryE.focus();
        await page.keyboard.type("Test issue with many comments");
        const details = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-new').shadowRoot.querySelector('paper-textarea[id="details"]')`);
        const detailsE = details.asElement();
        await detailsE.focus();
        await page.keyboard.type("Something goes here");
        const create = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-new').shadowRoot.querySelector('paper-button')`);
        const createE = create.asElement();
        await createE.click();
        await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view')`);
        // commenting
        const comment = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view').shadowRoot.querySelector('issue-new-comment').shadowRoot.querySelector('paper-textarea')`);
        const commentE = comment.asElement();
        const button = await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view').shadowRoot.querySelector('issue-new-comment').shadowRoot.querySelector('paper-button')`);
        const buttonE = button.asElement();
 
        for (var i = 0; i < 3; i++) {        
                await commentE.focus();
                await page.keyboard.type(`${i}`);
                await buttonE.click();
        }

        await page.waitForFunction(`document.querySelector('sit-app').shadowRoot.querySelector('issue-view').shadowRoot.querySelectorAll('issue-comment').length == 3`);
 
        // issues
        await page.goto('http://localhost:10803/search/length(comments || `[]`) > `2`', {waitUntil: 'networkidle0'});
        const screenshot = await page.screenshot({path: "getting_started/search.png"});
        web.kill();
        return browser.close();
}



async function example() {
        var web = child_process.spawn("sit-web", ["127.0.0.1:10800"]);

        await available(web);

        const browser = await launch();
        const page = await browser.newPage();
        await page.setViewport({width: 1024, height: 512});
        await page.goto('http://localhost:10800', {waitUntil: 'networkidle0'});
        const screenshot = await page.screenshot({path: "webui_example.png"});
        web.kill();
        return browser.close();
}




(async () => {
        // this could have been Promise.all but it
        // looks like it was causing timeouts in some
        // broweser (for some reason)
        await empty_screen();
        await new_issue();
        await search();
        await example();
        process.exit(0);
})();
