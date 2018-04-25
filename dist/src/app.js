/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/
"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : new P(function (resolve) { resolve(result.value); }).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
const UrlJoin = require("url-join");
const logger = require("./logger");
const express = require("express");
const GBConfigService_1 = require("../deploy/core.gbapp/services/GBConfigService");
const GBConversationalService_1 = require("../deploy/core.gbapp/services/GBConversationalService");
const GBMinService_1 = require("../deploy/core.gbapp/services/GBMinService");
const GBDeployer_1 = require("../deploy/core.gbapp/services/GBDeployer");
const GBCoreService_1 = require("../deploy/core.gbapp/services/GBCoreService");
const GBImporter_1 = require("../deploy/core.gbapp/services/GBImporter");
const analytics_gblib_1 = require("../deploy/analytics.gblib");
const core_gbapp_1 = require("../deploy/core.gbapp");
const kb_gbapp_1 = require("../deploy/kb.gbapp");
const security_gblib_1 = require("../deploy/security.gblib");
const index_1 = require("../deploy/admin.gbapp/index");
const customer_satisfaction_gbapp_1 = require("../deploy/customer-satisfaction.gbapp");
/**
 * General Bots open-core entry point.
 */
class GBServer {
    /** Program entry-point. */
    static run() {
        logger.info("Starting General Bots Open Core (Guaribas)...");
        // Creates a basic HTTP server that will serve several URL, one for each
        // bot instance. This allows the same server to attend multiple Bot on
        // the Marketplace until GB get serverless.
        let port = process.env.port || process.env.PORT || 4242;
        logger.info(`Starting GeneralBots HTTP server...`);
        let server = express();
        server.listen(port, () => {
            logger.info(`General Bots Server - RUNNING on ${port}...`);
            logger.info(`Starting instances...`);
            // Reads basic configuration, initialize minimal services.
            GBConfigService_1.GBConfigService.init();
            let core = new GBCoreService_1.GBCoreService();
            core.initDatabase(() => {
                // Boot a bot package if any.
                let deployer = new GBDeployer_1.GBDeployer(core, new GBImporter_1.GBImporter(core));
                // Build a minimal bot instance for each .gbot deployment.
                let conversationalService = new GBConversationalService_1.GBConversationalService(core);
                let minService = new GBMinService_1.GBMinService(core, conversationalService, deployer);
                let sysPackages = new Array();
                [index_1.GBAdminPackage, analytics_gblib_1.GBAnalyticsPackage, core_gbapp_1.GBCorePackage, security_gblib_1.GBSecurityPackage, kb_gbapp_1.GBKBPackage, customer_satisfaction_gbapp_1.GBCustomerSatisfactionPackage].forEach(e => {
                    logger.trace(`Loading sys package: ${e.name}...`);
                    let p = Object.create(e.prototype);
                    p.loadPackage(core, core.sequelize);
                    sysPackages.push(p);
                });
                (() => __awaiter(this, void 0, void 0, function* () {
                    try {
                        let appPackages = new Array();
                        yield minService.deployPackages(core, server, appPackages, sysPackages);
                        minService.buildMin(instance => {
                            logger.info(`Instance loaded: ${instance.botId}...`);
                        }, server, appPackages);
                    }
                    catch (err) {
                        logger.log(err);
                    }
                }))();
            });
            return core;
        });
    }
}
exports.GBServer = GBServer;
// First line to run.
GBServer.run();
//# sourceMappingURL=C:/Sources/opensource/BotServer/dist/src/app.js.map