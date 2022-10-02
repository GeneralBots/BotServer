/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots SSR support based on https://www.npmjs.com/package/ssr-for-bots. 
 */

'use strict';

const puppeteer = require('puppeteer');
const pluginStealth = require('puppeteer-extra-plugin-stealth');
import { NextFunction, Request, Response } from "express";
const Path = require('path');

// https://hackernoon.com/tips-and-tricks-for-web-scraping-with-puppeteer-ed391a63d952
// Dont download all resources, we just need the HTML
// Also, this is huge performance/response time boost
const blockedResourceTypes = [
    "image",
    "media",
    "font",
    "texttrack",
    "object",
    "beacon",
    "csp_report",
    "imageset",
];
// const whitelist = ["document", "script", "xhr", "fetch"];
const skippedResources = [
    "quantserve",
    "adzerk",
    "doubleclick",
    "adition",
    "exelator",
    "sharethrough",
    "cdn.api.twitter",
    "google-analytics",
    "googletagmanager",
    "google",
    "fontawesome",
    "facebook",
    "analytics",
    "optimizely",
    "clicktale",
    "mixpanel",
    "zedo",
    "clicksor",
    "tiqcdn",
];

const RENDER_CACHE = new Map();



async function createBrowser(profilePath): Promise<any> {

    let args = [
        '--ignore-certificate-errors',
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--window-size=1920,1080',
        "--disable-accelerated-2d-canvas",
        "--disable-gpu",
        "--disable-features=site-per-process"        
    ];

    if (profilePath){
         args.push(`--user-data-dir=${profilePath}`);        
    }

    const browser = await puppeteer.launch({
        args: args,
        ignoreHTTPSErrors: true,
        headless: false,
    });
    return browser;
}

async function recursiveFindInFrames(inputFrame, selector) {
    const frames = inputFrame.childFrames();
    const results = await Promise.all(
        frames.map(async frame => {
            const el = await frame.$(selector);
            if (el) return el;
            if (frame.childFrames().length > 0) {
                return await recursiveFindInFrames(frame, selector);
            }
            return null;
        })
    );
    return results.find(Boolean);
}




/**
 * https://developers.google.com/web/tools/puppeteer/articles/ssr#reuseinstance
 * @param {string} url URL to prerender.
 */
async function ssr(url: string, useCache: boolean, cacheRefreshRate: number) {
    if (RENDER_CACHE.has(url) && useCache) {
        const cached = RENDER_CACHE.get(url);
        if (
            Date.now() - cached.renderedAt > cacheRefreshRate &&
            !(cacheRefreshRate <= 0)
        ) {
            RENDER_CACHE.delete(url);
        } else {
            return {
                html: cached.html,
                status: 200,
            };
        }
    }
    const browser = await createBrowser(null);
    const stylesheetContents = {};

    try {
        const page = await browser.newPage();
        await page.setUserAgent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/83.0.4103.61 Safari/537.36"
        );
        await page.setRequestInterception(true);
        page.on("request", (request) => {
            const requestUrl = request.url().split("?")[0].split("#")[0];
            if (
                blockedResourceTypes.indexOf(request.resourceType()) !== -1 ||
                skippedResources.some((resource) => requestUrl.indexOf(resource) !== -1)
            ) {
                request.abort();
            } else {
                request.continue();
            }
        });

        page.on("response", async (resp) => {
            const responseUrl = resp.url();
            const sameOrigin = new URL(responseUrl).origin === new URL(url).origin;
            const isStylesheet = resp.request().resourceType() === "stylesheet";
            if (sameOrigin && isStylesheet) {
                stylesheetContents[responseUrl] = await resp.text();
            }
        });

        const response = await page.goto(url, {
            timeout: 120000,
            waitUntil: "networkidle0",
        });

        const sleep = ms => {
            return new Promise(resolve => {
                setTimeout(resolve, ms);
            });
        };
        await sleep(45000);

        // Inject <base> on page to relative resources load properly.
        await page.evaluate((url) => {
            const base = document.createElement("base");
            base.href = url;
            // Add to top of head, before all other resources.
            document.head.prepend(base);
        }, url);

        // Remove scripts and html imports. They've already executed.
        await page.evaluate(() => {
            const elements = document.querySelectorAll('script, link[rel="import"]');
            elements.forEach((e) => {
                e.remove();
            });
        });

        // Replace stylesheets in the page with their equivalent <style>.
        await page.$$eval(
            'link[rel="stylesheet"]',
            (links, content) => {
                links.forEach((link: any) => {
                    const cssText = content[link.href];
                    if (cssText) {
                        const style = document.createElement("style");
                        style.textContent = cssText;
                        link.replaceWith(style);
                    }
                });
            },
            stylesheetContents
        );

        const html = await page.content();

        // Close the page we opened here (not the browser).
        await page.close();
        if (useCache) {
            RENDER_CACHE.set(url, { html, renderedAt: Date.now() });
        }
        return { html, status: response!.status() };
    } catch (e) {
        const html = e.toString();
        console.warn({ message: `URL: ${url} Failed with message: ${html}` });
        return { html, status: 500 };
    } finally {
        await browser.close();
    }
}

function clearCache() {
    RENDER_CACHE.clear();
}

interface Options {
    prerender?: Array<string>;
    exclude?: Array<string>;
    useCache?: boolean;
    cacheRefreshRate?: number;
}

function ssrForBots(
    options: Options = {
        prerender: [], // Array containing the user-agents that will trigger the ssr service
        exclude: [], // Array containing paths and/or extentions that will be excluded from being prerendered by the ssr service
        useCache: true, // Variable that determins if we will use page caching or not
        cacheRefreshRate: 86400 // Seconds of which the cache will be kept alive, pass 0 or negative value for infinite lifespan
    }
) {
    let applyOptions = Object.assign(
        {
            prerender: [], // Array containing the user-agents that will trigger the ssr service
            exclude: [], // Array containing paths and/or extentions that will be excluded from being prerendered by the ssr service
            useCache: true, // Variable that determins if we will use page caching or not
            cacheRefreshRate: 86400 // Seconds of which the cache will be kept alive, pass 0 or negative value for infinite lifespan
        },
        options
    );

    // Default user agents
    const prerenderArray = [
        "bot",
        "googlebot",
        "Chrome-Lighthouse",
        "DuckDuckBot",
        "ia_archiver",
        "bingbot",
        "yandex",
        "baiduspider",
        "Facebot",
        "facebookexternalhit",
        "facebookexternalhit/1.1",
        "twitterbot",
        "rogerbot",
        "linkedinbot",
        "embedly",
        "quora link preview",
        "showyoubot",
        "outbrain",
        "pinterest",
        "slackbot",
        "vkShare",
        "W3C_Validator",
    ];

    // default exclude array
    const excludeArray = [".xml", ".ico", ".txt", ".json"];

    function ssrOnDemand(req: Request, res: Response, next: NextFunction) {
        Promise.resolve(() => {
            return true;
        })
            .then(async () => {
                const userAgent: string = req.headers["user-agent"] || "";

                const prerender = new RegExp(
                    [...prerenderArray, ...applyOptions.prerender].join("|").slice(0, -1),
                    "i"
                ).test(userAgent);

                const exclude = !new RegExp(
                    [...excludeArray, ...applyOptions.exclude].join("|").slice(0, -1)
                ).test(req.originalUrl);

                if (req.originalUrl && prerender && exclude) {
                    const { html, status } = await ssr(
                        req.protocol + "://" + req.get("host") + req.originalUrl,
                        applyOptions.useCache,
                        applyOptions.cacheRefreshRate
                    );
                    return res.status(status).send(html);
                } else {
                    return next();
                }
            })
            .catch(next);
    }

    return ssrOnDemand;
}


export { createBrowser, ssr, clearCache, ssrForBots };
