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

const Swagger = require('swagger-client');
const fs = require('fs');
const { promisify } = require('util');
import { GBLog, GBMinInstance, GBService } from 'botlib';
import { GBServer } from '../../../src/app';
import { SecService } from '../../security.gbapp/services/SecService';
const hubspot = require('@hubspot/api-client');


/**
 * Support for Hub Spot XRM.
 */
export class HubSpotServices extends GBService {

  public static conversationIds = {};
  public pollInterval = 5000;

  public botId: string;
  public min: GBMinInstance;
  private key: any;

  constructor(
    min: GBMinInstance,
    botId,
    key
  ) {
    super();

    this.min = min;
    this.botId = botId;
    this.key = key;

  }
  public static async asyncForEach(array, callback) {
    for (let index = 0; index < array.length; index++) {
      await callback(array[index], index, array);
    }
  }

  public async getActiveTasks(): Promise<[]> {

    const client = new hubspot.Client({ apiKey: this.key });
    let properties = ['hs_task_subject', 'hubspot_owner_id', 'hs_task_status', 'hs_task_priority'];
    const pageSize = 100;
    let list;
    list = [];

    let r = await client.crm.objects.basicApi.getPage("TASK", pageSize, 0, properties);
    list = list.concat(r.body.results);

    while (r.body.results && r.body.results.length === pageSize) {
      r = await client.crm.objects.basicApi.getPage("TASK", pageSize, r.body.paging.next.after, properties);
      list = list.concat(r.body.results);
    }

    let final;
    final = [];
    list.forEach(e => {
      if (e.properties.hs_task_status === "NOT_STARTED") {
        e['status'] = e.properties.hs_task_status;
        e['title'] = e.properties.hs_task_subject;
        e['ownerId'] = e.properties.hubspot_owner_id;
        e['priority'] = e.properties.hs_task_priority;

        final.push(e);
      }
    });


    return final;
  }
}