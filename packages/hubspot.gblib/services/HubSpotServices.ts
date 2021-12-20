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

  public async addDealNote(name, note)
  {

  }

  public async createDeal(dealName, contact, company, amount) {
    const dealObj = {
      properties: {
        dealname: dealName,
        dealstage: 'appointmentscheduled',
        pipeline: 'default',
        amount: amount
      },
    }
    const contactObj = {
      properties: {
        firstname: contact
      },
    }
    const companyObj = {
      properties: {
        name: company,
      },
    }

    const hubspotClient = new hubspot.Client({ apiKey: this.key });
    const createDealResponse = await hubspotClient.crm.deals.basicApi.create(dealObj);
    const createContactResponse = await hubspotClient.crm.contacts.basicApi.create(contactObj);
    const createCompanyResponse = await hubspotClient.crm.companies.basicApi.create(companyObj);

    await hubspotClient.crm.deals.associationsApi.create(
      createDealResponse.body.id,
      'contacts',
      createContactResponse.body.id,
      'deal_to_contact'
    )

    await hubspotClient.crm.deals.associationsApi.create(
      createDealResponse.body.id,
      'companies',
      createCompanyResponse.body.id,
      'deal_to_company'
    )

    return createDealResponse.body;

  }


  public async createContact(firstName, lastName, domain, companyName) {
    const contactObj = {
      properties: {
        firstname: firstName,
        lastname: lastName,
      },
    }
    const companyObj = {
      properties: {
        domain: domain,
        name: companyName,
      },
    }

    const hubspotClient = new hubspot.Client({ apiKey: this.key })
    const createContactResponse = await hubspotClient.crm.contacts.basicApi.create(contactObj)
    const createCompanyResponse = await hubspotClient.crm.companies.basicApi.create(companyObj)

    return await hubspotClient.crm.companies.associationsApi.create(
      createCompanyResponse.body.id,
      'contacts',
      createContactResponse.body.id,
      'company_to_contact'
    )
  }

  public async searchContact(query) {
    const client = new hubspot.Client({ apiKey: this.key });
    const filter = { propertyName: 'createdate', operator: 'GTE', value: Date.now() - 30 * 60000 }
    const filterGroup = { filters: [filter] }
    const sort = JSON.stringify({ propertyName: 'createdate', direction: 'DESCENDING' })

    const properties = ['createdate', 'firstname', 'lastname']
    const limit = 100
    const after = 0

    const publicObjectSearchRequest = {
      filterGroups: [filterGroup],
      sorts: [sort],
      query,
      properties,
      limit,
      after,
    }

    const result = await client.crm.contacts.searchApi.doSearch(publicObjectSearchRequest)
    console.log(JSON.stringify(result.body))
    return result.body;
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