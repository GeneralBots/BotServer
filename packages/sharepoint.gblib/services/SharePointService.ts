/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
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
 * @fileoverview General Bots server core.
 */

'use strict';

import { HttpHeaders, HttpMethods, ServiceClient, WebResource } from '@azure/ms-rest-js';
import { CognitiveServicesManagementClient } from 'azure-arm-cognitiveservices';
import { ResourceManagementClient, SubscriptionClient } from 'azure-arm-resource';
import { SearchManagementClient } from 'azure-arm-search';
import { SqlManagementClient } from 'azure-arm-sql';
import { WebSiteManagementClient } from 'azure-arm-website';
//tslint:disable-next-line:no-submodule-imports
import { AppServicePlan, Site, SiteConfigResource, SiteLogsConfig, SiteSourceControl } from 'azure-arm-website/lib/models';
import { GBLog, IGBInstallationDeployer, IGBInstance } from 'botlib';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GBCorePackage } from '../../core.gbapp';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
const MicrosoftGraph = require("@microsoft/microsoft-graph-client");

const Spinner = require('cli-spinner').Spinner;
// tslint:disable-next-line: no-submodule-imports
import * as simplegit from 'simple-git/promise';
const git = simplegit();

// tslint:disable-next-line:no-submodule-imports
import { CognitiveServicesAccount } from 'azure-arm-cognitiveservices/lib/models';
import urlJoin = require('url-join');
const iconUrl = 'https://github.com/pragmatismo-io/BotServer/blob/master/docs/images/generalbots-logo-squared.png';
const publicIp = require('public-ip');
const WebSiteResponseTimeout = 900;

/**
 * Service facade for SharePoint Online.
 */
export class GBSharePointService {

    public async downloadFolder(localPath: string, siteUrl: string, folderUrl: string, username: string, password: string) {
        const { sppull } = require("sppull");

        const context = {
            siteUrl: siteUrl,
            creds: {
                username: username,
                password: password
            }
        };

        const options = {
            spRootFolder: folderUrl,
            dlRootFolder: localPath
        };

        return await sppull(context, options);
    }

}
