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
const msRestAzure = require("ms-rest-azure");
import {
  ResourceManagementClient,
  SubscriptionClient
} from "azure-arm-resource";
import { WebSiteManagementClient } from "azure-arm-website";
import { SqlManagementClient } from "azure-arm-sql";
import { CognitiveServicesManagementClient } from "azure-arm-cognitiveservices";
import { CognitiveServicesAccount } from "azure-arm-cognitiveservices/lib/models";
import { SearchManagementClient } from "azure-arm-search";
import { WebResource, ServiceClient } from "ms-rest-js";
import * as simplegit from "simple-git/promise";
import { AppServicePlan } from "azure-arm-website/lib/models";
import { GBConfigService } from "deploy/core.gbapp/services/GBConfigService";
const scanf = require("scanf");
const git = simplegit();
const logger = require("../../../src/logger");
const UrlJoin = require("url-join");
const PasswordGenerator = require("strict-password-generator").default;
const iconUrl =
  "https://github.com/pragmatismo-io/BotServer/blob/master/docs/images/generalbots-logo-squared.png";
  
  
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

  constructor(credentials, subscriptionId, location) {
    super();
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
    this.location= location;
  }

  public static async getSubscriptions(credentials) {
    let subscriptionClient = new SubscriptionClient.default(credentials);
    return subscriptionClient.subscriptions.list();
  }

  public async deployFarm(
    name: string,
    location: string
  ): Promise<IGBInstance> {
    let instance = new IGBInstance();

    logger.info(`Creating Deploy...`);
    await this.createDeploy(name, location);

    logger.info(`Creating Server...`);
    let serverFarm = await this.createHostingPlan(
      name,
      `${name}-server-plan`,
      location
    );
    await this.createServer(serverFarm.id, name, `${name}-server`, location);

    let administratorLogin = AzureDeployerService.getRndAdminAccount();
    let administratorPassword = AzureDeployerService.getRndPassword();

    logger.info(`Creating Storage...`);
    let storageServerName = `${name}-storage`;
    await this.createStorageServer(
      name,
      `${storageServerName}-server`,
      administratorLogin,
      administratorPassword,
      storageServerName,
      location
    );

    await this.createStorage(
      name,
      storageServerName,
      `${name}-storage`,
      location
    );
    instance.storageUsername = administratorLogin;
    instance.storagePassword = administratorPassword;
    instance.storageName = storageServerName;
    instance.storageDialect = "mssql";
    instance.storageServerName = storageServerName;

    logger.info(`Creating Search...`);
    let search = await this.createSearch(name, `${name}-search`, location);
    instance.searchHost = "generalbots.search.windows.net";
    instance.searchIndex = "azuresql-index";
    instance.searchIndexer = "azuresql-indexer";
    instance.searchKey = "0FF1CE27564C208555A22B6E278289813";

    logger.info(`Creating NLP...`);
    let nlp = await this.createNLP(name, `${name}-nlp`, location);
    let keys = await this.cognitiveClient.accounts.listKeys(name, nlp.name);
    instance.nlpEndpoint = nlp.endpoint;
    instance.nlpKey = keys.key1;
    instance.nlpAppId = "0ff1ceb4f-96a4-4bdb-b2d5-3ea462ddb773";

    logger.info(`Creating Speech...`);
    let speech = await this.createSpeech(name, `${name}-speech`, location);
    keys = await this.cognitiveClient.accounts.listKeys(name, speech.name);
    instance.speechKeyEndpoint = speech.endpoint;
    instance.speechKey = keys.key1;

    logger.info(`Creating SpellChecker...`);
    let spellChecker = await this.createSpellChecker(
      name,
      `${name}-spellchecker`,
      location
    );
    keys = await this.cognitiveClient.accounts.listKeys(
      name,
      spellChecker.name
    );
    instance.spellCheckerKey = keys.key1;
    instance.spellCheckerEndpoint = spellChecker.endpoint;

    logger.info(`Creating Text Analytics...`);
    let textAnalytics = await this.createTextAnalytics(
      name,
      `${name}-textanalytics`,
      location
    );
    keys = await this.cognitiveClient.accounts.listKeys(
      name,
      textAnalytics.name
    );
    instance.textAnalyticsEndpoint = textAnalytics.endpoint;
    instance.textAnalyticsKey = keys.key1;

    return instance;
  }

  public async deployBot(instance, name, endpoint, nlpAppId, nlpKey, subscriptionId) {

    logger.info(`Creating Bot...`);
    await this.internalDeployBot(
      this.accessToken,
      name,
      name,
      name,
      "General BootBot",
      endpoint,
      "global",
      nlpAppId,
      nlpKey,
      subscriptionId
    );
    instance.webchatKey = "********";
    instance.marketplaceId = "0ff1ce73-0aea-442a-a222-dcc340eca294";
    instance.marketplacePassword = "************";

    return instance;
  }

  private async dangerouslyDeleteDeploy(name) {
    return this.resourceClient.resourceGroups.deleteMethod(name);
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
    req.headers["x-ms-client-request-id"] = msRestAzure.generateUuid();
    req.headers["Authorization"] = "Bearer " + accessToken;

    let httpClient = new ServiceClient();
    let res = await httpClient.sendRequest(req);
  }

  private async internalDeployBot(
    accessToken,
    botId,
    group,
    name,
    description,
    endpoint,
    location,
    nlpAppId,
    nlpKey,
    subscriptionId
  ) {
    let baseUrl = `https://management.azure.com/`;

    let appId = msRestAzure.generateUuid();

    let parameters = {
      parameters: {
        location: location,
        sku: {
          name: "F0"
        },
        name: name,
        id: botId,
        kind: "sdk",
        properties: {
          description: description,
          displayName: name,
          endpoint: endpoint,
          iconUrl: iconUrl,
          luisAppIds: [nlpAppId],
          luisKey: nlpKey,
          msaAppId: appId
        }
      }
    };

    let query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
      this.provider
    }/botServices/${botId}?api-version=2017-12-01`;
    let requestUrl = UrlJoin(baseUrl, query);

    let req = new WebResource();
    req.method = "PUT";
    req.url = requestUrl;
    req.headers = {};
    req.headers["Content-Type"] = "application/json";
    req.headers["accept-language"] = "*";
    req.headers["Authorization"] = "Bearer " + accessToken;

    let requestContent = JSON.stringify(parameters);
    req.body = requestContent;

    let httpClient = new ServiceClient();
    let res = await httpClient.sendRequest(req);

    return JSON.parse(res.bodyAsJson as string);
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
    // * 'Bing.Autosuggest.v7', 'Bing.CustomSearch',
    // * 'Bing.Search.v7', 'Bing.Speech', 'Bing.SpellCheck.v7', 'ComputerVision',
    // * 'ContentModerator', 'CustomSpeech', 'CustomVision.Prediction',
    // * 'CustomVision.Training', 'Emotion', 'Face', 'LUIS', 'QnAMaker',
    // * 'SpeakerRecognition', 'SpeechTranslation', 'TextAnalytics',
    // * 'TextTranslation', 'WebLM'

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

  private async createDeploy(name, location) {
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
    return this.webSiteClient.webApps.createOrUpdateConfiguration(
      group,
      name,
      siteConfig
    );
  }

  private deleteDeploy(name) {
    return this.resourceClient.resourceGroups.deleteMethod(name);
  }

  async deployGeneralBotsToAzure() {
    let status = await git.status();
  }

  private static getRndAdminAccount() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: true,
      lowerCaseAlpha: true,
      number: true,
      specialCharacter: true,
      minimumLength: 8,
      maximumLength: 8
    };
    let password = passwordGenerator.generatePassword(options);
    return `sa${password}`;
  }

  private static getRndPassword() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: true,
      lowerCaseAlpha: true,
      number: true,
      specialCharacter: true,
      minimumLength: 8,
      maximumLength: 8
    };
    let password = passwordGenerator.generatePassword(options);
    return password;
  }
  
  static async ensureDeployer() {
    
    // Tries do get information from .env file otherwise asks in command-line.

    let username = GBConfigService.get("CLOUD_USERNAME");
    let password = GBConfigService.get("CLOUD_PASSWORD");
    let subscriptionId = GBConfigService.get("CLOUD_SUBSCRIPTIONID");
    let cloudLocation = GBConfigService.get("CLOUD_LOCATION");

    // No .env so asks for cloud credentials to start a new farm.

    if (!username || !password || !subscriptionId || !cloudLocation) {
      process.stdout.write(
        "FIRST RUN: A empty enviroment is detected. Please, enter credentials to create a new General Bots Farm."
      );
    }

    let retriveUsername = () => {
      if (!username) {
        process.stdout.write("CLOUD_USERNAME:");
        username = scanf("%s");
      }
    };

    let retrivePassword = () => {
      if (!password) {
        process.stdout.write("CLOUD_PASSWORD:");
        password = scanf("%s");
      }
    };

    while (!username) {
      retriveUsername();
    }

    while (!password) {
      retrivePassword();
    }

    // Connects to the cloud and retrives subscriptions.

    let credentials = await msRestAzure.loginWithUsernamePassword(
      username,
      password
    );
    let list = await AzureDeployerService.getSubscriptions(credentials);

    let map = {};
    let index = 1;
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
    if (!subscriptionId) {
      while (!subscriptionIndex) {
        retrieveSubscription();
      }
      subscriptionId = map[subscriptionIndex].subscriptionId;
    }


    let retriveLocation = () => {
      if (!location) {
        process.stdout.write("CLOUD_LOCATION:");
        location = scanf("%s");
      }
    };

    while (!location) {
      retriveLocation();
    }

    return new AzureDeployerService(credentials, subscriptionId, location);
  }
  
}
