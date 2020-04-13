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
import { GBLog, IGBInstallationDeployer, IGBInstance, IGBDeployer } from 'botlib';
import { GBAdminService } from '../../../packages/admin.gbapp/services/GBAdminService';
import { GBCorePackage } from '../../../packages/core.gbapp';
import { GBConfigService } from '../../../packages/core.gbapp/services/GBConfigService';
import { GBDeployer } from '../../../packages/core.gbapp/services/GBDeployer';
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
 * Deployer for Microsoft cloud.
 */
export class AzureDeployerService implements IGBInstallationDeployer {
  public apiVersion = '2017-12-01';
  public defaultEndPoint = 'http://localhost:4242';
  public instance: IGBInstance;
  public resourceClient: ResourceManagementClient.ResourceManagementClient;
  public webSiteClient: WebSiteManagementClient;
  public storageClient: SqlManagementClient;
  public cognitiveClient: CognitiveServicesManagementClient;
  public searchClient: SearchManagementClient;
  public provider = 'Microsoft.BotService';
  public subscriptionClient: SubscriptionClient.SubscriptionClient;
  public accessToken: string;
  public location: string;
  public subscriptionId: string;
  public farmName: any;
  public deployer: IGBDeployer;

  constructor(deployer: IGBDeployer) {
    this.deployer = deployer;
  }

  public static async createInstance(deployer: GBDeployer): Promise<AzureDeployerService> {

    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const credentials = await GBAdminService.getADALCredentialsFromUsername(username, password);
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    const service = new AzureDeployerService(deployer);
    service.initServices(credentials, subscriptionId);

    return service;
  }

  private static createRequestObject(url: string, accessToken: string, verb: HttpMethods, body: string) {
    const req = new WebResource();
    req.method = verb;
    req.url = url;
    req.headers.set('Content-Type', 'application/json');
    req.headers.set('accept-language', '*');
    req.headers.set('Authorization', `Bearer ${accessToken}`);
    req.body = body;

    return req;
  }

  public async getSubscriptions(credentials) {
    const subscriptionClient = new SubscriptionClient.default(credentials);

    return subscriptionClient.subscriptions.list();
  }

  public getKBSearchSchema(indexName) {
    return {
      name: indexName,
      fields: [
        {
          name: 'questionId',
          type: 'Edm.String',
          searchable: false,
          filterable: false,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: true
        },
        {
          name: 'subject1',
          type: 'Edm.String',
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'subject2',
          type: 'Edm.String',
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'subject3',
          type: 'Edm.String',
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'subject4',
          type: 'Edm.String',
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'content',
          type: 'Edm.String',
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'answerId',
          type: 'Edm.Int32',
          searchable: false,
          filterable: false,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'instanceId',
          type: 'Edm.Int32',
          searchable: false,
          filterable: true,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: 'packageId',
          type: 'Edm.Int32',
          searchable: false,
          filterable: true,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        }
      ],
      scoringProfiles: [],
      defaultScoringProfile: undefined,
      corsOptions: undefined
    };
  }

  public async botExists(botId, group) {
    const baseUrl = `https://management.azure.com/`;
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const httpClient = new ServiceClient();

    const query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
      this.provider
      }/botServices/${botId}?api-version=${this.apiVersion}`;
    const url = urlJoin(baseUrl, query);
    const req = AzureDeployerService.createRequestObject(url, accessToken, 'GET', undefined);
    const res = await httpClient.sendRequest(req);
    // CHECK
    if (!JSON.parse(res.bodyAsText).id) {
      return false;
    } else {
      return true;
    }
  }

  public async updateBotProxy(botId, group, endpoint) {
    const baseUrl = `https://management.azure.com/`;
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const httpClient = new ServiceClient();

    const parameters = {
      properties: {
        endpoint: endpoint
      }
    };

    const query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
      this.provider
      }/botServices/${botId}?api-version=${this.apiVersion}`;
    const url = urlJoin(baseUrl, query);
    const req = AzureDeployerService.createRequestObject(url, accessToken, 'PATCH', JSON.stringify(parameters));
    const res = await httpClient.sendRequest(req);
    // CHECK
    if (!JSON.parse(res.bodyAsText).id) {
      throw res.bodyAsText;
    }
    GBLog.info(`Bot proxy updated at: ${endpoint}.`);
  }

  public async updateBot(botId: string, group: string, name: string,
    description: string, endpoint: string) {
    const baseUrl = `https://management.azure.com/`;
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const httpClient = new ServiceClient();

    const parameters = {
      properties: {
        description: description,
        displayName: name,
        endpoint: endpoint,
        iconUrl: iconUrl
      }
    };

    const query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
      this.provider
      }/botServices/${botId}?api-version=${this.apiVersion}`;
    const url = urlJoin(baseUrl, query);
    const req = AzureDeployerService.createRequestObject(url, accessToken, 'PATCH', JSON.stringify(parameters));
    const res = await httpClient.sendRequest(req);
    // CHECK
    if (!JSON.parse(res.bodyAsText).id) {
      throw res.bodyAsText;
    }
    GBLog.info(`Bot proxy updated at: ${endpoint}.`);
  }

  public async deleteBot(botId: string, group) {
    const baseUrl = `https://management.azure.com/`;
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const httpClient = new ServiceClient();

    const query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
      this.provider
      }/botServices/${botId}?api-version=${this.apiVersion}`;
    const url = urlJoin(baseUrl, query);
    const req = AzureDeployerService.createRequestObject(url, accessToken, 'DELETE', undefined);
    const res = await httpClient.sendRequest(req);

    if (res.bodyAsText !== "") {
      throw res.bodyAsText;
    }
    GBLog.info(`Bot ${botId} was deleted from the provider.`);
  }

  public async openStorageFirewall(groupName, serverName) {
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    const credentials = await GBAdminService.getADALCredentialsFromUsername(username, password);
    const storageClient = new SqlManagementClient(credentials, subscriptionId);

    const ip = await publicIp.v4();
    let params = {
      startIpAddress: ip,
      endIpAddress: ip
    };
    await storageClient.firewallRules.createOrUpdate(groupName, serverName, 'gb', params);

    // AllowAllWindowsAzureIps must be created that way, so the Azure Search can 
    // access SQL Database to index its contents.

    params = {
      startIpAddress: '0.0.0.0',
      endIpAddress: '0.0.0.0'
    };
    await storageClient.firewallRules.createOrUpdate(groupName, serverName, 'AllowAllWindowsAzureIps', params);

  }

  public async deployFarm(
    proxyAddress: string,
    instance: IGBInstance,
    credentials,
    subscriptionId: string
  ): Promise<IGBInstance> {
    const culture = 'en-us';

    this.initServices(credentials, subscriptionId);
    const spinner = new Spinner('%s');
    spinner.start();
    spinner.setSpinnerString('|/-\\');
    let keys: any;
    const name = instance.botId;

    GBLog.info(`Deploying Deploy Group (It may take a few minutes)...`);
    await this.createDeployGroup(name, instance.cloudLocation);

    GBLog.info(`Deploying NLP...`);
    const nlp = await this.createNLP(name, `${name}-nlp`, instance.cloudLocation);
    keys = await this.cognitiveClient.accounts.listKeys(name, nlp.name);
    const nlpAppId = await this.createNLPService(name, name, instance.cloudLocation, culture, instance.nlpAuthoringKey);

    instance.nlpEndpoint = urlJoin(nlp.endpoint, 'apps');
    instance.nlpKey = keys.key1;
    instance.nlpAppId = nlpAppId;

    GBLog.info(`Deploying Bot Server...`);
    const serverFarm = await this.createHostingPlan(name, `${name}-server-plan`, instance.cloudLocation);
    const serverName = `${name}-server`;
    await this.createServer(serverFarm.id, name, serverName, instance.cloudLocation);

    GBLog.info(`Deploying Bot Storage...`);
    const administratorLogin = `sa${GBAdminService.getRndReadableIdentifier()}`;
    const administratorPassword = GBAdminService.getRndPassword();
    const storageServer = `${name.toLowerCase()}-storage-server`;
    const storageName = `${name}-storage`;
    await this.createStorageServer(name, storageServer, administratorLogin,
      administratorPassword, storageServer, instance.cloudLocation
    );
    await this.createStorage(name, storageServer, storageName, instance.cloudLocation);
    instance.storageUsername = administratorLogin;
    instance.storagePassword = administratorPassword;
    instance.storageName = storageName;
    instance.storageDialect = 'mssql';
    instance.storageServer = storageServer;

    GBLog.info(`Deploying Search...`);
    const searchName = `${name}-search`.toLowerCase();
    await this.createSearch(name, searchName, instance.cloudLocation);
    const searchKeys = await this.searchClient.adminKeys.get(name, searchName);
    instance.searchHost = `${searchName}.search.windows.net`;
    instance.searchIndex = 'azuresql-index';
    instance.searchIndexer = 'azuresql-indexer';
    instance.searchKey = searchKeys.primaryKey;
    this.deployer.rebuildIndex(instance, this.getKBSearchSchema(instance.searchIndex));

    GBLog.info(`Deploying Speech...`);
    const speech = await this.createSpeech(name, `${name}-speech`, instance.cloudLocation);
    keys = await this.cognitiveClient.accounts.listKeys(name, speech.name);
    instance.speechEndpoint = speech.endpoint;
    instance.speechKey = keys.key1;

    GBLog.info(`Deploying SpellChecker...`);
    const spellChecker = await this.createSpellChecker(name, `${name}-spellchecker`);
    keys = await this.cognitiveClient.accounts.listKeys(name, spellChecker.name);
    instance.spellcheckerKey = keys.key1;
    instance.spellcheckerEndpoint = spellChecker.endpoint;

    GBLog.info(`Deploying Text Analytics...`);
    const textAnalytics = await this.createTextAnalytics(name, `${name}-textanalytics`, instance.cloudLocation);
    keys = await this.cognitiveClient.accounts.listKeys(name, textAnalytics.name);

    instance.textAnalyticsEndpoint = textAnalytics.endpoint.replace(`/text/analytics/v2.0`, '');
    instance.textAnalyticsKey = keys.key1;

    // NLP

    GBLog.info(`Deploying Bot...`);
    instance.botEndpoint = this.defaultEndPoint;

    instance = await this.internalDeployBot(
      instance, this.accessToken, name, name, name, 'General BootBot',
      `${proxyAddress}/api/messages/${name}`, 'global',
      instance.nlpAppId, instance.nlpKey, instance.marketplaceId, instance.marketplacePassword,
      instance.cloudSubscriptionId
    );

    GBLog.info('Updating server environment variables...');
    await this.updateWebisteConfig(name, serverName, serverFarm.id, instance);

    spinner.stop();

    GBLog.info('Opening your browser with default.gbui...');
    const opn = require('opn');
    opn(`https://${serverName}.azurewebsites.net`);

    return instance;
  }

  public async deployToCloud(
    title: string,
    username: string,
    password: string,
    cloudLocation: string,
    authoringKey: string,
    appId: string,
    appPassword: string,
    subscriptionId: string
  ) {
    const instance = <IGBInstance>{};

    instance.botId = title;
    instance.cloudUsername = username;
    instance.cloudPassword = password;
    instance.cloudSubscriptionId = subscriptionId;
    instance.cloudLocation = cloudLocation;
    instance.nlpAuthoringKey = authoringKey;
    instance.marketplaceId = appId;
    instance.marketplacePassword = appPassword;
    instance.adminPass = GBAdminService.getRndPassword();

    const credentials = await GBAdminService.getADALCredentialsFromUsername(username, password);
    // tslint:disable-next-line:no-http-string
    const url = `https://${instance.botId}.azurewebsites.net`;
    this.deployFarm(url, instance, credentials, subscriptionId);
  }

  /**
   * @see https://github.com/Azure/azure-rest-api-specs/blob/master/specification/botservice/resource-manager/Microsoft.BotService/preview/2017-12-01/botservice.json
   */
  public async internalDeployBot(
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
  ): Promise<IGBInstance> {
    return new Promise(async (resolve, reject) => {
      const baseUrl = `https://management.azure.com/`;
      await this.registerProviders(subscriptionId, baseUrl, accessToken);

      instance.marketplaceId = appId;
      instance.marketplacePassword = appPassword;
      instance.engineName = GBCorePackage['CurrentEngineName'];

      const parameters = {
        location: location,
        sku: {
          name: 'F0'
        },
        name: botId,
        kind: 'bot',
        properties: {
          description: description,
          displayName: name,
          endpoint: endpoint,
          iconUrl: iconUrl,
          luisAppIds: [nlpAppId],
          luisKey: nlpKey,
          msaAppId: appId,
          msaAppPassword: appPassword,
          enabledChannels: ['webchat', "skype"],//, "facebook"],
          configuredChannels: ['webchat' , "skype"]//, "facebook"]
        }
      };

      const httpClient = new ServiceClient();
      let query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/${
        this.provider
        }/botServices/${botId}?api-version=${this.apiVersion}`;
      let url = urlJoin(baseUrl, query);
      let req = AzureDeployerService.createRequestObject(url, accessToken, 'PUT', JSON.stringify(parameters));
      const res = await httpClient.sendRequest(req);
      if (!JSON.parse(res.bodyAsText).id) {
        reject(res.bodyAsText);

        return;
      }

      setTimeout(async () => {
        try {
          //tslint:disable-next-line:max-line-length
          query = `subscriptions/${subscriptionId}/resourceGroups/${group}/providers/Microsoft.BotService/botServices/${botId}/channels/WebChatChannel/listChannelWithKeys?api-version=${
            this.apiVersion
            }`;
          url = urlJoin(baseUrl, query);
          req = AzureDeployerService.createRequestObject(url, accessToken, 'POST', JSON.stringify(parameters));
          const resChannel = await httpClient.sendRequest(req);
          const key = JSON.parse(resChannel.bodyAsText).properties.properties.sites[0].key;
          instance.webchatKey = key;
          instance.whatsappBotKey = key;
          resolve(instance);
        } catch (error) {
          reject(error);
        }
      }, 60000);
    });
  }

  public async syncBotServerRepository(group, name) {
    await this.webSiteClient.webApps.syncRepository(group, name);
  }

  public initServices(credentials: any, subscriptionId: string) {
    this.resourceClient = new ResourceManagementClient.default(credentials, subscriptionId);
    this.webSiteClient = new WebSiteManagementClient(credentials, subscriptionId);
    this.storageClient = new SqlManagementClient(credentials, subscriptionId);
    this.cognitiveClient = new CognitiveServicesManagementClient(credentials, subscriptionId);
    this.searchClient = new SearchManagementClient(credentials, subscriptionId);
    this.accessToken = credentials.tokenCache._entries[0].accessToken;
  }

  private async createStorageServer(group, name, administratorLogin, administratorPassword, serverName, location) {
    const params = {
      location: location,
      administratorLogin: administratorLogin,
      administratorLoginPassword: administratorPassword,
      fullyQualifiedDomainName: `${serverName}.database.windows.net`
    };

    return await this.storageClient.servers.createOrUpdate(group, name, params);
  }

  public async createApplication(token: string, name: string) {
    return new Promise<string>((resolve, reject) => {
      let client = MicrosoftGraph.Client.init({
        authProvider: done => {
          done(null, token);
        }
      });
      const app = {
        displayName: name
      };

      client.api(`/applications`).post(app, (err, res) => {
        if (err) {
          reject(err)
        }
        else {
          resolve(res);
        }
      });
    });
  }

  public async createApplicationSecret(token: string, appId: string) {
    return new Promise<string>((resolve, reject) => {
      let client = MicrosoftGraph.Client.init({
        authProvider: done => {
          done(null, token);
        }
      });
      const body = {
        passwordCredential: {
          displayName: "General Bots Generated"
        }
      };

      client.api(`/applications/${appId}/addPassword`).post(body, (err, res) => {
        if (err) {
          reject(err)
        }
        else {
          resolve(res.secretText);
        }
      });
    });
  }

  private async registerProviders(subscriptionId, baseUrl, accessToken) {
    const query = `subscriptions/${subscriptionId}/providers/${this.provider}/register?api-version=2018-02-01`;
    const requestUrl = urlJoin(baseUrl, query);

    const req = new WebResource();
    req.method = 'POST';
    req.url = requestUrl;
    req.headers = <any>{};
    req.headers['Content-Type'] = 'application/json; charset=utf-8';
    req.headers['accept-language'] = '*';
    (req.headers as any).Authorization = `Bearer ${accessToken}`;
  }

  private async createNLPService(
    name: string,
    description: string,
    location: string,
    culture: string,
    authoringKey: string
  ) {
    const parameters = {
      name: name,
      description: description,
      culture: culture
    };

    const body = JSON.stringify(parameters);
    const apps = await this.makeNlpRequest(location, authoringKey, undefined, 'GET', 'apps');
    const app = JSON.parse(apps.bodyAsText).filter(x => x.name === name)[0];
    let id: string;
    if (!app) {
      const res = await this.makeNlpRequest(location, authoringKey, body, 'POST', 'apps');
      id = res.bodyAsText;
    } else {
      id = app.id;
    }

    return id.replace(/\'/gi, '');
  }

  private async makeNlpRequest(
    location: string,
    authoringKey: string,
    body: string,
    method: HttpMethods,
    resource: string
  ) {
    const req = new WebResource();
    req.method = method;
    req.url = `https://${location}.api.cognitive.microsoft.com/luis/api/v2.0/${resource}`;
    req.headers.set('Content-Type', 'application/json');
    req.headers.set('accept-language', '*');
    req.headers.set('Ocp-Apim-Subscription-Key', authoringKey);
    req.body = body;
    const httpClient = new ServiceClient();

    return await httpClient.sendRequest(req);
  }

  private async createSearch(group, name, location) {
    const params = {
      sku: { name: 'free' },
      location: location
    };

    return await this.searchClient.services.createOrUpdate(group, name, params);
  }

  private async createStorage(group, serverName, name, location) {
    const params = {
      sku: { name: 'Free' },
      createMode: 'Default',
      location: location
    };

    return await this.storageClient.databases.createOrUpdate(group, serverName, name, params);
  }

  private async createCognitiveServices(group, name, location, kind): Promise<CognitiveServicesAccount> {
    const params = {
      sku: { name: 'F0' },
      createMode: 'Default',
      location: location,
      kind: kind,
      properties: {}
    };

    return await this.cognitiveClient.accounts.create(group, name, params);
  }

  private async createSpeech(group, name, location): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(group, name, location, 'SpeechServices');
  }

  private async createNLP(group, name, location): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(group, name, location, 'LUIS');
  }

  private async createSpellChecker(group, name): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(group, name, 'global', 'Bing.SpellCheck.v7');
  }

  private async createTextAnalytics(group, name, location): Promise<CognitiveServicesAccount> {
    return await this.createCognitiveServices(group, name, location, 'TextAnalytics');
  }

  private async createDeployGroup(name, location) {
    const params = { location: location };

    return await this.resourceClient.resourceGroups.createOrUpdate(name, params);
  }

  private async createHostingPlan(group, name, location): Promise<AppServicePlan> {
    const params = {
      serverFarmWithRichSkuName: name,
      location: location,
      sku: {
        name: 'F1',
        capacity: 1,
        tier: 'Free'
      }
    };

    return await this.webSiteClient.appServicePlans.createOrUpdate(group, name, params);
  }

  private async createServer(farmId, group, name, location) {
    const parameters: Site = {
      location: location,
      serverFarmId: farmId,

      siteConfig: {
        nodeVersion: GBAdminService.getNodeVersion(),
        detailedErrorLoggingEnabled: true,
        requestTracingEnabled: true
      }
    };
    const server = await this.webSiteClient.webApps.createOrUpdate(group, name, parameters);

    const siteLogsConfig: SiteLogsConfig = {
      applicationLogs: {
        fileSystem: { level: 'Error' }
      }
    };
    await this.webSiteClient.webApps.updateDiagnosticLogsConfig(group, name, siteLogsConfig);

    const souceControlConfig: SiteSourceControl = {
      repoUrl: 'https://github.com/GeneralBots/BotServer.git',
      branch: 'master',
      isManualIntegration: true,
      isMercurial: false,
      deploymentRollbackEnabled: false
    };

    await this.webSiteClient.webApps.createOrUpdateSourceControl(group, name, souceControlConfig);

    return server;
  }

  private async updateWebisteConfig(group, name, serverFarmId, instance: IGBInstance) {

    const parameters: Site = {
      location: instance.cloudLocation,
      serverFarmId: serverFarmId,
      siteConfig: {
        appSettings: [
          { name: 'WEBSITES_CONTAINER_START_TIME_LIMIT', value: `${WebSiteResponseTimeout}` },
          { name: 'WEBSITE_NODE_DEFAULT_VERSION', value: GBAdminService.getNodeVersion() },
          { name: 'ADDITIONAL_DEPLOY_PATH', value: `` },
          { name: 'ADMIN_PASS', value: `${instance.adminPass}` },
          { name: 'BOT_ID', value: `${instance.botId}` },
          { name: 'CLOUD_SUBSCRIPTIONID', value: `${instance.cloudSubscriptionId}` },
          { name: 'CLOUD_LOCATION', value: `${instance.cloudLocation}` },
          { name: 'CLOUD_GROUP', value: `${instance.botId}` },
          { name: 'CLOUD_USERNAME', value: `${instance.cloudUsername}` },
          { name: 'CLOUD_PASSWORD', value: `${instance.cloudPassword}` },
          { name: 'MARKETPLACE_ID', value: `${instance.marketplaceId}` },
          { name: 'MARKETPLACE_SECRET', value: `${instance.marketplacePassword}` },
          { name: 'NLP_AUTHORING_KEY', value: `${instance.nlpAuthoringKey}` },
          { name: 'STORAGE_DIALECT', value: `${instance.storageDialect}` },
          { name: 'STORAGE_SERVER', value: `${instance.storageServer}.database.windows.net` },
          { name: 'STORAGE_NAME', value: `${instance.storageName}` },
          { name: 'STORAGE_USERNAME', value: `${instance.storageUsername}` },
          { name: 'STORAGE_PASSWORD', value: `${instance.storagePassword}` },
          { name: 'STORAGE_SYNC', value: `true` }]

      }
    };

    return await this.webSiteClient.webApps.createOrUpdate(group, name, parameters);
  }

}
