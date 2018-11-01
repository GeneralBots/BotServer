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

"use strict";

import { GBService, IGBInstance } from "botlib";
import {
  ResourceManagementClient,
  SubscriptionClient
} from "azure-arm-resource";
import { WebSiteManagementClient } from "azure-arm-website";
import { SqlManagementClient } from "azure-arm-sql";
import { CognitiveServicesManagementClient } from "azure-arm-cognitiveservices";
import { CognitiveServicesAccount } from "azure-arm-cognitiveservices/lib/models";
import { SearchManagementClient } from "azure-arm-search";
import { WebResource, ServiceClient, HttpMethods } from "ms-rest-js";
import * as simplegit from "simple-git/promise";
import { AppServicePlan } from "azure-arm-website/lib/models";
import { GBConfigService } from "../../../packages/core.gbapp/services/GBConfigService";
import { GBAdminService } from "../../../packages/admin.gbapp/services/GBAdminService";

const Spinner = require("cli-spinner").Spinner;
const scanf = require("scanf");
const git = simplegit();
const logger = require("../../../src/logger");
const UrlJoin = require("url-join");
const iconUrl =
  "https://github.com/pragmatismo-io/BotServer/blob/master/docs/images/generalbots-logo-squared.png";
const publicIp = require("public-ip");

export class AzureDeployerService extends GBService {
  instance: IGBInstance;
  resourceClient: ResourceManagementClient.ResourceManagementClient;
  webSiteClient: WebSiteManagementClient;
  storageClient: SqlManagementClient;
  cognitiveClient: CognitiveServicesManagementClient;
  searchClient: SearchManagementClient;
  provider = "Microsoft.BotService";
  subscriptionClient: SubscriptionClient.SubscriptionClient;
  accessToken: string;
  location: string;
  public subscriptionId: string;
  static apiVersion = "2017-12-01";
  farmName: any;

  public static async getSubscriptions(credentials) {
    let subscriptionClient = new SubscriptionClient.default(credentials);
    return subscriptionClient.subscriptions.list();
  }

  public async deployFarm(proxyAddress: string): Promise<IGBInstance> {
    let culture = "en-us";

    // Tries do get information from .env file otherwise asks in command-line.

    let instance: IGBInstance = {};
    instance = await this.ensureConfiguration(instance);
    instance.marketplacePassword = GBAdminService.getRndPassword();

    let spinner = new Spinner("%s");
    spinner.start();
    spinner.setSpinnerString("|/-\\");

    let keys: any;
    let name = instance.botId;

    logger.info(`Deploying Deploy Group (It may take a few minutes)...`);
    await this.createDeployGroup(name, instance.cloudLocation);

    instance = await this.deployBootBot(
      instance,
      name,
      `${proxyAddress}/api/messages/${name}`,
      instance.nlpAppId,
      instance.nlpKey,
      instance.cloudSubscriptionId
    );


    logger.info(`Deploying Bot Server...`);
    let serverFarm = await this.createHostingPlan(
      name,
      `${name}-server-plan`,
      instance.cloudLocation
    );
    await this.createServer(
      serverFarm.id,
      name,
      `${name}-server`,
      instance.cloudLocation
    );

    logger.info(`Deploying Bot Storage...`);
    let administratorLogin = `sa${GBAdminService.getRndReadableIdentifier()}`;
    let administratorPassword = GBAdminService.getRndPassword();
    let storageServer = `${name}-storage-server`;
    let storageName = `${name}-storage`;
    await this.createStorageServer(
      name,
      storageServer,
      administratorLogin,
      administratorPassword,
      storageServer,
      instance.cloudLocation
    );
    await this.createStorage(
      name,
      storageServer,
      storageName,
      instance.cloudLocation
    );
    instance.storageUsername = administratorLogin;
    instance.storagePassword = administratorPassword;
    instance.storageName = storageName;
    instance.storageDialect = "mssql";
    instance.storageServer = storageServer;

    logger.info(`Deploying Search...`);
    let searchName = `${name}-search`;
    await this.createSearch(name, searchName, instance.cloudLocation);
    let searchKeys = await this.searchClient.queryKeys.listBySearchService(
      name,
      searchName
    );
    instance.searchHost = `${searchName}.search.windows.net`;
    instance.searchIndex = "azuresql-index";
    instance.searchIndexer = "azuresql-indexer";
    instance.searchKey = searchKeys[0].key;

    logger.info(`Deploying Speech...`);
    let speech = await this.createSpeech(
      name,
      `${name}-speech`,
      instance.cloudLocation
    );
    keys = await this.cognitiveClient.accounts.listKeys(name, speech.name);
    instance.speechKeyEndpoint = speech.endpoint;
    instance.speechKey = keys.key1;

    logger.info(`Deploying SpellChecker...`);
    let spellChecker = await this.createSpellChecker(
      name,
      `${name}-spellchecker`,
      instance.cloudLocation
    );
    keys = await this.cognitiveClient.accounts.listKeys(
      name,
      spellChecker.name
    );
    instance.spellCheckerKey = keys.key1;
    instance.spellCheckerEndpoint = spellChecker.endpoint;

    logger.info(`Deploying Text Analytics...`);
    let textAnalytics = await this.createTextAnalytics(
      name,
      `${name}-textanalytics`,
      instance.cloudLocation
    );
    keys = await this.cognitiveClient.accounts.listKeys(
      name,
      textAnalytics.name
    );
    instance.textAnalyticsEndpoint = textAnalytics.endpoint;
    instance.textAnalyticsKey = keys.key1;

    logger.info(`Deploying NLP...`);
    let nlp = await this.createNLP(name, `${name}-nlp`, instance.cloudLocation);
    keys = await this.cognitiveClient.accounts.listKeys(name, nlp.name);
    let nlpAppId = await this.createLUISApp(
      name,
      name,
      instance.cloudLocation,
      culture,
      instance.nlpAuthoringKey
    );

    instance.nlpEndpoint = nlp.endpoint;
    instance.nlpKey = keys.key1;
    instance.nlpAppId = nlpAppId;

    logger.info(`Deploying Bot...`);
    instance = await this.deployBootBot(
      instance,
      name,
      `${proxyAddress}/api/messages/${name}`,
      instance.nlpAppId,
      instance.nlpKey,
      instance.cloudSubscriptionId
    );

    spinner.stop();
    return instance;
  }

  public async openStorageFirewall(groupName, serverName) {
    let username = GBConfigService.get("CLOUD_USERNAME");
    let password = GBConfigService.get("CLOUD_PASSWORD");
    let subscriptionId = GBConfigService.get("CLOUD_SUBSCRIPTIONID");

    let credentials = await GBAdminService.getADALCredentialsFromUsername(
      username,
      password
    );
    let storageClient = new SqlManagementClient(credentials, subscriptionId);

    let ip = await publicIp.v4();
    let params = {
      startIpAddress: ip,
      endIpAddress: ip
    };

    await storageClient.firewallRules.createOrUpdate(
      groupName,
      serverName,
      "gb",
      params
    );
  }

  private async ensureConfiguration(instance: IGBInstance) {
    let username = GBConfigService.get("CLOUD_USERNAME");
    let password = GBConfigService.get("CLOUD_PASSWORD");
    let subscriptionId = GBConfigService.get("CLOUD_SUBSCRIPTIONID");
    let location = GBConfigService.get("CLOUD_LOCATION");
    let botId = GBConfigService.get("BOT_ID");

    // No .env so asks for cloud credentials to start a new farm.
    if (!username || !password || !subscriptionId || !location || !botId) {
      process.stdout.write(
        "A empty enviroment is detected. To start automatic deploy, please enter some information:\n"
      );
    }
    let retriveUsername = () => {
      if (!username) {
        process.stdout.write("CLOUD_USERNAME:");
        username = scanf("%s").replace(/(\n|\r)+$/, "");
      }
    };
    let retrivePassword = () => {
      if (!password) {
        process.stdout.write("CLOUD_PASSWORD:");
        password = scanf("%s").replace(/(\n|\r)+$/, "");
      }
    };
    let retrieveBotId = () => {
      if (!botId) {
        process.stdout.write(
          "Bot Id must only contain lowercase letters, digits or dashes, cannot start or end with or contain consecutive dashes and is limited from 4 to 42 characters long.\n"
        );
        process.stdout.write("BOT_ID:");
        botId = scanf("%s").replace(/(\n|\r)+$/, "");
      }
    };
    let authoringKey = GBConfigService.get("NLP_AUTHORING_KEY");
    let retriveAuthoringKey = () => {
      if (!authoringKey) {
        process.stdout.write(
          "Due to this opened issue: https://github.com/Microsoft/botbuilder-tools/issues/550\n"
        );
        process.stdout.write("Please enter your LUIS Authoring Key:");
        authoringKey = scanf("%s").replace(/(\n|\r)+$/, "");
      }
    };
    while (!authoringKey) {
      retriveAuthoringKey();
    }
    while (!botId) {
      retrieveBotId();
    }
    while (!username) {
      retriveUsername();
    }
    while (!password) {
      retrivePassword();
    }

    // Connects to the cloud and retrives subscriptions.

    let credentials = await GBAdminService.getADALCredentialsFromUsername(
      username,
      password
    );
    if (!subscriptionId) {
      let map = {};
      let index = 1;
      let list = await AzureDeployerService.getSubscriptions(credentials);
      list.forEach(element => {
        console.log(
          `${index}: ${element.displayName} (${element.subscriptionId})`
        );
        map[index++] = element;
      });
      let subscriptionIndex;
      let retrieveSubscription = () => {
        if (!subscriptionIndex) {
          process.stdout.write("CLOUD_SUBSCRIPTIONID (type a number):");
          subscriptionIndex = scanf("%d");
        }
      };
      while (!subscriptionIndex) {
        retrieveSubscription();
      }
      subscriptionId = map[subscriptionIndex].subscriptionId;
    }
    let retriveLocation = () => {
      if (!location) {
        process.stdout.write("CLOUD_LOCATION (eg. 'westus'):");
        location = scanf("%s");
      }
    };
    while (!location) {
      retriveLocation();
    }

    // Prepares the first instance on bot farm.

    instance.botId = botId;
    instance.cloudUsername = username;
    instance.cloudPassword = password;
    instance.cloudSubscriptionId = subscriptionId;
    instance.cloudLocation = location;
    instance.nlpAuthoringKey = authoringKey;
    instance.adminPass = GBAdminService.getRndPassword();

    this.resourceClient = new ResourceManagementClient.default(
      credentials,
      subscriptionId
    );
    this.webSiteClient = new WebSiteManagementClient(
      credentials,
      subscriptionId
    );
    this.storageClient = new SqlManagementClient(credentials, subscriptionId);
    this.cognitiveClient = new CognitiveServicesManagementClient(
      credentials,
      subscriptionId
    );

    this.searchClient = new SearchManagementClient(credentials, subscriptionId);
    this.accessToken = credentials.tokenCache._entries[0].accessToken;

    return instance;
  }

  public async deployBootBot(
    instance,
    botId,
    endpoint,
    nlpAppId,
    nlpKey,
    subscriptionId
  ) {
    let appId = GBConfigService.get("MSAPP_ID");
    let appPassword = GBConfigService.get("MSAPP_PASSWORD");

    if (!appId || !appPassword) {
      process.stdout.write(
        "Sorry, this part cannot be automated yet due to Microsoft schedule, please go to https://apps.dev.microsoft.com/portal/register-app to generate manually an App ID and App Secret.\n"
      );
    }

    let retriveAppId = () => {
      if (!appId) {
        process.stdout.write("Generated Application Id (MSAPP_ID):");
        appId = scanf("%s").replace(/(\n|\r)+$/, "");
      }
    };

    let retriveAppPassword = () => {
      if (!appPassword) {
        process.stdout.write("Generated Password (MSAPP_PASSWORD):");
        appPassword = scanf("%s").replace(/(\n|\r)+$/, "");
      }
    };

    retriveAppId();
    retriveAppPassword();

    await this.internalDeployBot(
      instance,
      this.accessToken,
      botId,
      botId,
      botId,
      "General BootBot",
      endpoint,
      "global",
      nlpAppId,
      nlpKey,
      appId,
      appPassword,
      subscriptionId
    );
    instance.marketplaceId = appId;
    instance.marketplacePassword = appPassword;
    instance.botId = botId;

    return instance;
  }

  private async createStorageServer(
    group,
    name,
    administratorLogin,
    administratorPassword,
    serverName,
    location
  ) {
    var params = {
      location: location,
      administratorLogin: administratorLogin,
      administratorLoginPassword: administratorPassword,
      fullyQualifiedDomainName: `${serverName}.database.windows.net`
    };

    return this.storageClient.servers.createOrUpdate(group, name, params);
  }

  private async registerProviders(subscriptionId, baseUrl, accessToken) {
    let query = `subscriptions/${subscriptionId}/providers/${
      this.provider
    }/register?api-version=2018-02-01`;
    let requestUrl = UrlJoin(baseUrl, query);

    let req = new WebResource();
    req.method = "POST";
    req.url = requestUrl;
    req.headers = {};
    req.headers["Content-Type"] = "application/json; charset=utf-8";
    req.headers["accept-language"] = "*";
    req.headers["Authorization"] = "Bearer " + accessToken;

    let httpClient = new ServiceClient();
    let res = await httpClient.sendRequest(req);
    // TODO: Check res for error.
  }
  /**
   * @see https://github.com/Azure/azure-rest-api-specs/blob/master/specification/botservice/resource-manager/Microsoft.BotService/preview/2017-12-01/botservice.json
   */
  private async internalDeployBot(
    instance,
    accessToken,
    botId,
    name,
    group,
    description,
    endpoint,
    location,
    nlpAppId,
    nlpKey,
    appId,
    appPassword,
    subscriptionId
  ) {
    return new Promise(async (resolve, reject) => {
      let baseUrl = `https://management.azure.com/`;
      await this.registerProviders(subscriptionId, baseUrl, accessToken);

      instance.marketplaceId = appId;
      instance.marketplacePassword = appPassword;

      let parameters = {
        location: location,
        sku: {
          name: "F0"
        },
        name: botId,
        kind: "bot",
        properties: {
          description: description,
          displayName: name,
          endpoint: endpoint,
          iconUrl: iconUrl,
          //luisAppIds: [nlpAppId],
          //luisKey: nlpKey,
          msaAppId: appId,
          msaAppPassword: appPassword,
          //enabledChannels: ["webchat"], // , "skype", "facebook"],
          configuredChannels: ["webchat"] // , "skype", "facebook"]
        }
      };

      let httpClient = new ServiceClient();
      let query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
        this.provider
      }/botServices/${botId}?api-version=${AzureDeployerService.apiVersion}`;
      let url = UrlJoin(baseUrl, query);
      let req = this.createRequestObject(
        url,
        accessToken,
        "PUT",
        JSON.stringify(parameters)
      );
      let res = await httpClient.sendRequest(req);
      if (!(res.bodyAsJson as any).id) {
        reject(res.bodyAsText);
        return;
      }

      logger.info(`Bot creation request done waiting for key generation...`);
      resolve(instance);

      setTimeout(async () => {
        try {
          query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/Microsoft.BotService/botServices/${botId}/channels/WebChatChannel/listChannelWithKeys?api-version=${
            AzureDeployerService.apiVersion
          }`;
          url = UrlJoin(baseUrl, query);
          req = this.createRequestObject(
            url,
            accessToken,
            "GET",
            JSON.stringify(parameters)
          );
          let resChannel = await httpClient.sendRequest(req);
          let key = (resChannel.bodyAsJson as any).properties.properties
            .sites[0].key;
          instance.webchatKey = key;
          resolve(instance);
        } catch (error) {
          reject(error);
        }
      }, 20000);
    });
  }

  public async updateBotProxy(botId, group, endpoint) {
    let baseUrl = `https://management.azure.com/`;
    let username = GBConfigService.get("CLOUD_USERNAME");
    let password = GBConfigService.get("CLOUD_PASSWORD");
    let subscriptionId = GBConfigService.get("CLOUD_SUBSCRIPTIONID");

    let accessToken = await GBAdminService.getADALTokenFromUsername(
      username,
      password
    );
    let httpClient = new ServiceClient();

    let parameters = {
      properties: {
        endpoint: endpoint
      }
    };

    let query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
      this.provider
    }/botServices/${botId}?api-version=${AzureDeployerService.apiVersion}`;
    let url = UrlJoin(baseUrl, query);
    let req = this.createRequestObject(
      url,
      accessToken,
      "PATCH",
      JSON.stringify(parameters)
    );
    let res = await httpClient.sendRequest(req);
    if (!(res.bodyAsJson as any).id) {
      throw res.bodyAsText;
    }
    logger.info(`Bot proxy updated at: ${endpoint}.`);
  }

  private createRequestObject(
    url: string,
    accessToken: string,
    verb: HttpMethods,
    body: string
  ) {
    let req = new WebResource();
    req.method = verb;
    req.url = url;
    req.headers = {};
    req.headers["Content-Type"] = "application/json";
    req.headers["accept-language"] = "*";
    req.headers["Authorization"] = "Bearer " + accessToken;
    req.body = body;
    return req;
  }

  private async createLUISApp(
    name: string,
    description: string,
    location: string,
    culture: string,
    authoringKey: string
  ) {
    let parameters = {
      name: name,
      description: description,
      culture: culture
    };

    let body = JSON.stringify(parameters);
    let apps = await this.makeNlpRequest(
      location,
      authoringKey,
      null,
      "GET",
      "apps"
    );
    let app = (apps.bodyAsJson as any).filter(x => x.name == name)[0];
    let id: string;
    if (!app) {
      let res = await this.makeNlpRequest(
        location,
        authoringKey,
        body,
        "POST",
        "apps"
      );
      id = res.bodyAsText;
    } else {
      id = app.id;
    }

    return id;
  }

  private async makeNlpRequest(
    location: string,
    authoringKey: string,
    body: string,
    method: HttpMethods,
    resource: string
  ) {
    let req = new WebResource();
    req.method = method;
    req.url = `https://${location}.api.cognitive.microsoft.com/luis/api/v2.0/${resource}`;
    req.headers = {};
    req.headers["Content-Type"] = "application/json";
    req.headers["accept-language"] = "*";
    req.headers["Ocp-Apim-Subscription-Key"] = authoringKey;
    req.body = body;
    let httpClient = new ServiceClient();
    let res = await httpClient.sendRequest(req);
    return res;
  }

  private async createSearch(group, name, location) {
    var params = {
      sku: { name: "free" },
      location: location
    };

    return this.searchClient.services.createOrUpdate(group, name, params);
  }

  private async createStorage(group, serverName, name, location) {
    var params = {
      sku: { name: "Free" },
      createMode: "Default",
      location: location
    };

    return this.storageClient.databases.createOrUpdate(
      group,
      serverName,
      name,
      params
    );
  }

  private async createCognitiveServices(
    group,
    name,
    location,
    kind
  ): Promise<CognitiveServicesAccount> {
    let params = {
      sku: { name: "F0" },
      createMode: "Default",
      location: location,
      kind: kind,
      properties: {}
    };

    return await this.cognitiveClient.accounts.create(group, name, params);
  }

  private async createSpeech(
    group,
    name,
    location
  ): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(
      group,
      name,
      location,
      "SpeechServices"
    );
  }

  private async createNLP(
    group,
    name,
    location
  ): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(group, name, location, "LUIS");
  }

  private async createSpellChecker(
    group,
    name,
    location
  ): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(
      group,
      name,
      "global",
      "Bing.SpellCheck.v7"
    );
  }

  private async createTextAnalytics(
    group,
    name,
    location
  ): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(
      group,
      name,
      location,
      "TextAnalytics"
    );
  }

  private async createDeployGroup(name, location) {
    var params = { location: location };
    return this.resourceClient.resourceGroups.createOrUpdate(name, params);
  }

  private async createHostingPlan(
    group,
    name,
    location
  ): Promise<AppServicePlan> {
    let params = {
      serverFarmWithRichSkuName: name,
      location: location,
      sku: {
        name: "F1",
        capacity: 1,
        tier: "Free"
      }
    };

    return this.webSiteClient.appServicePlans.createOrUpdate(
      group,
      name,
      params
    );
  }

  private async createServer(farmId, group, name, location) {
    var parameters = {
      location: location,
      serverFarmId: farmId
    };
    return this.webSiteClient.webApps.createOrUpdate(group, name, parameters);
  }

  private async updateWebisteConfig(group, serverFarmId, name, location) {
    var siteConfig = {
      location: location,
      serverFarmId: serverFarmId,
      numberOfWorkers: 1,
      phpVersion: "5.5"
    };

    // TODO: Copy .env to app settings.

    return this.webSiteClient.webApps.createOrUpdateConfiguration(
      group,
      name,
      siteConfig
    );
  }

  async deployGeneralBotsToAzure() {
    let status = await git.status();
    // TODO: Copy github to webapp.
  }

  static getKBSearchSchema(indexName) {
    return {
      name: indexName,
      fields: [
        {
          name: "questionId",
          type: "Edm.String",
          searchable: false,
          filterable: false,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: true
        },
        {
          name: "subject1",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "subject2",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "subject3",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "subject4",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "content",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "answerId",
          type: "Edm.Int32",
          searchable: false,
          filterable: false,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "instanceId",
          type: "Edm.Int32",
          searchable: false,
          filterable: true,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "packageId",
          type: "Edm.Int32",
          searchable: false,
          filterable: true,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        }
      ],
      scoringProfiles: [],
      defaultScoringProfile: null,
      corsOptions: null
    };
  }
}
