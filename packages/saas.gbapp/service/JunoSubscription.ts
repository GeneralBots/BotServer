/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';
import fs from 'fs/promises'; 
import { HttpMethods, HttpOperationResponse, ServiceClient, WebResource } from '@azure/ms-rest-js';
import { GBLog } from 'botlib';
import urlJoin from 'url-join';
// tslint:disable-next-line: no-require-imports
const Juno = require('juno-payment-node');
var FormData = require('form-data');
export class JunoSubscription {
  /**
   * The host this service will call REST API through VPN.
   */
  public host: string = process.env.SAAS_JUNO_HOST;

  /**
   * Creates a HTTP request object to make REST calls.
   */
  private async getAuthorizationToken(): Promise<string> {
    GBLog.info( `JunoAPI: Getting Auth Token from API...`);
    const httpClient = new ServiceClient();

    const req: WebResource = new WebResource();
    req.method = 'POST';
    req.url = JunoSubscription.getAuthUrl();
    req.body = 'grant_type=client_credentials';
    req.headers.set('Content-Type', 'application/x-www-form-urlencoded');
    req.headers.set(
      'Authorization',
      'Basic ' +
        new Buffer(JunoSubscription.getClientId() + ':' + JunoSubscription.getClientSecret()).toString('base64')
    );

    const res = await httpClient.sendRequest(req);

    GBLog.info( `JunoAPI: Response from Authorization API ${res.bodyAsText}`);

    return res.parsedBody.access_token;
  }

  /**
   * Creates a HTTP request object to make REST calls.
   */
  private async setupWebhook(): Promise<string> {
    GBLog.info( `JunoAPI: Setting Webhook...`);
    const httpClient = new ServiceClient();

    const host = process.env.BOT_URL;
    const url = `${host}/store.gbapp/payment_notification`;

    const body = {
      url: '',
      eventTypes: ['PAYMENT_NOTIFICATION']
    };

    const req: WebResource = new WebResource();
    req.method = 'POST';
    req.url = urlJoin(JunoSubscription.getResourceUrl(), 'notifications', 'webhooks');

    req.body = body;
    req.headers.set('Content-Type', 'application/x-www-form-urlencoded');
    req.headers.set(
      'Authorization',
      'Basic ' +
        new Buffer(JunoSubscription.getClientId() + ':' + JunoSubscription.getClientSecret()).toString('base64')
    );

    const res = await httpClient.sendRequest(req);

    GBLog.info( `JunoAPI: Response from Authorization API ${res.bodyAsText}`);

    return res.parsedBody.access_token;
  }

  /**
   * Creates a HTTP request object to make REST calls.
   */
  private static createRequestObject(
    token: string,
    url: string,
    verb: HttpMethods,
    body: string,
    headers: any,
    externalAccountToken = undefined
  ): WebResource {
    const req: WebResource = new WebResource();
    req.method = verb;
    req.url = url;

    req.headers.set('Content-Type', 'application/json;charset=UTF-8');
    req.headers.set('Authorization', `Bearer ${token}`);
    req.headers.set('X-Api-Version', 2);
    req.headers.set(
      'X-Resource-Token',
      externalAccountToken ? externalAccountToken : JunoSubscription.getJunoPrivateKey()
    );

    if (headers !== undefined) {
      // tslint:disable-next-line: typedef
      headers.forEach(e => {
        req.headers.set(e.name, e.value);
      });
    }
    req.body = body;

    return req;
  }

  public async PayByBoleto(
    name: string,
    document: string,
    email: string,
    phone: string,
    amount: number
  ): Promise<string> {
    let charge = await this.createCharge(name, document, email, phone, amount, 'BOLETO');

    return charge;
  }

  public async PayByCC(
    name: string,
    document: string,
    email: string,
    phone: string,
    ccNumber: string,
    ccExpiresOnMonth: string,
    ccExpiresOnYear: string,
    ccCode: string,
    amount: number
  ): Promise<string> {
    let externalSubscriptionId = '1';

    let charge = await this.createCharge(name, document, email, phone, amount, 'CREDIT_CARD');

    let ccHash = await this.getCardHash(ccNumber, name, ccCode, ccExpiresOnMonth, ccExpiresOnYear);
    let ccId = await this.getCreditCardId(ccHash);
    let final = await this.makePayment(ccId, ccHash, charge.Id, email);

    return externalSubscriptionId;
  }

  /***
   * Get active users available to the workflow process.
   */
  public async createDigitalAccount(
    name,
    document,
    email,
    birthDate,
    phone,
    businessArea,
    linesOfBusiness,
    number: string,
    digit: string,
    bank: string
  ) {
    GBLog.info( `JunoAPI: Calling createDigitalAccount API...`);

    let token = await this.getAuthorizationToken();

    const httpClient = new ServiceClient();

    const url = urlJoin(JunoSubscription.getResourceUrl(), 'digital-accounts');
    const req = JunoSubscription.createRequestObject(token, url, 'POST', '', undefined);
    const res = await httpClient.sendRequest(req);

    let json = {
      type: 'PAYMENT',
      name: name,
      document: document,
      email: email,
      birthDate: birthDate,
      phone: phone
    };

    GBLog.info( `JunoAPI: Response from createDigitalAccount ${res.bodyAsText}`);

    return res.parsedBody;
  }

  private async createCharge(name, document, email, phone, amount, paymentType) {
    GBLog.info( `JunoAPI: Calling createCharge API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), 'charges');

    let json = {
      charge: {
        description: 'string',
        amount: amount,
        paymentTypes: [paymentType]
      },
      billing: {
        name: name,
        document: document,
        email: email,
        phone: phone,
        notify: true
      }
    };

    const req = JunoSubscription.createRequestObject(token, url, 'POST', JSON.stringify(json), undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from createCharge ${res.bodyAsText}`);

    return res.parsedBody._embedded.charges[0];
  }

  public async createPlan(name, amount) {
    GBLog.info( `JunoAPI: Calling createPlan API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/plans');

    let json = {
      name: name,
      amount: amount
    };

    const req = JunoSubscription.createRequestObject(token, url, 'POST', JSON.stringify(json), undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from createPlan ${res.bodyAsText}`);

    return res.parsedBody;
  }

  private async createSubscription(
    dueDay,
    planId,
    description,
    email,
    creditCardId,
    creditCardHash,
    street,
    number,
    city,
    state,
    postCode,
    partnerAccountToken
  ) {
    GBLog.info( `JunoAPI: Calling createSubscription API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/subscriptions');

    let json = {
      dueDay: dueDay,
      planId: planId,
      chargeDescription: description,
      creditCardDetails: {
        creditCardId: creditCardId,
        creditCardHash: creditCardHash
      },
      billing: {
        email: email,
        address: {
          street: street,
          number: number,
          city: city,
          state: state,
          postCode: postCode
        }
      }
    };

    if (partnerAccountToken) {
      json['split'] = [
        {
          recipientToken: this.getAuthorizationToken(),
          percentage: 90,
          amountRemainder: true,
          chargeFee: true
        },
        {
          recipientToken: partnerAccountToken,
          percentage: 10,
          amountRemainder: false,
          chargeFee: true
        }
      ];
    }

    const req = JunoSubscription.createRequestObject(token, url, 'POST', JSON.stringify(json), undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from createSubscription ${res.bodyAsText}`);

    return res.parsedBody;
  }

  public async getBusinessAreas() {
    GBLog.info( `JunoAPI: Calling getBusinessAreas API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/data/business-areas');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from getBusiness ${res.bodyAsText}`);

    return res.parsedBody._embedded.businessAreas;
  }

  public async getBanks() {
    GBLog.info( `JunoAPI: Calling getBanks API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/data/banks');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from getBanks ${res.bodyAsText}`);

    return res.parsedBody._embedded.banks;
  }

  public async getCompanyTypes() {
    GBLog.info( `JunoAPI: Calling getCompanyTypes API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/data/company-types');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from getCompanyTypes ${res.bodyAsText}`);

    return res.parsedBody._embedded.banks;
  }

  public async getAccountPublicKey(externalAccountToken) {
    GBLog.info( `JunoAPI: Calling getAccountPublicKey API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/credentials/public-key');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined, externalAccountToken);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from getAccountPublicKey ${res.bodyAsText}`);

    return res.bodyAsText;
  }

  public async listAccountDocuments(externalAccountToken: string) {
    GBLog.info( `JunoAPI: Calling listAccountDocuments API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/documents');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined, externalAccountToken);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from listAccountDocuments ${res.bodyAsText}`);

    return res.parsedBody._embedded.documents;
  }

  public async getAccountDocumentProperties(externalAccountToken: string, id: string) {
    GBLog.info( `JunoAPI: Calling getAccountDocumentProperties API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), `/documents/${id}`);

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined, externalAccountToken);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from getAccountDocumentProperties ${res.bodyAsText}`);

    return res.parsedBody;
  }

  public async sendAccountDocument(externalAccountToken: string, id: string, file: string) {
    GBLog.info( `JunoAPI: Calling sendAccountDocument API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), `/documents/${id}/files`);
    var form = new FormData();
    form.append('file', await fs.readFile(file));

    const req = JunoSubscription.createRequestObject(
      token,
      url,
      'POST',
      form.getBuffer(),
      form.getHeaders(),
      externalAccountToken
    );

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from sendAccountDocument ${res.bodyAsText}`);

    return res.parsedBody;
  }

  public async getAccountBalance(externalAccountToken) {
    GBLog.info( `JunoAPI: Calling getAccountBalance API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/balance');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined, externalAccountToken);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from getAccountBalance ${res.bodyAsText}`);

    return res.parsedBody;
  }

  public async getAccount(externalAccountToken: string, id: string): Promise<string> {
    GBLog.info( `JunoAPI: Calling Get Digital Accounts API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/digital-accounts');

    const req = JunoSubscription.createRequestObject(token, url, 'GET', undefined, undefined, externalAccountToken);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from Get Digital Accounts ${res.bodyAsText}`);

    return res.parsedBody.id;
  }

  public async getCreditCardId(ccHash: string): Promise<string> {
    GBLog.info( `JunoAPI: Calling tokenizeCreditCard API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/credit-cards/tokenization');

    let json = {
      creditCardHash: ccHash
    };

    const req = JunoSubscription.createRequestObject(token, url, 'POST', JSON.stringify(json), undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from tokenizeCreditCard ${res.bodyAsText}`);

    return res.parsedBody.creditCardId;
  }

  public async makePayment(ccId, ccHash, chargeId, email): Promise<string> {
    GBLog.info( `JunoAPI: Calling makePayment API...`);

    let token = await this.getAuthorizationToken();
    const httpClient = new ServiceClient();
    const url = urlJoin(JunoSubscription.getResourceUrl(), '/payments');

    let json = {
      chargeId: chargeId,
      billing: {
        email: email
        // address: {
        //     street: street,
        //     number: number,
        //     complement: complement,
        //     neighborhood: neighborhood,
        //     city: city,
        //     state: state,
        //     postCode: postCode
        // }
      },
      creditCardDetails: {
        creditCardId: ccId,
        creditCardHash: ccHash
      }
    };

    const req = JunoSubscription.createRequestObject(token, url, 'POST', JSON.stringify(json), undefined);

    const res = await httpClient.sendRequest(req);
    GBLog.info( `JunoAPI: Response from makePayment ${res.bodyAsText}`);

    return res.parsedBody._embedded.charges[0];
  }

  private static isProd() {
    return process.env.SAAS_JUNO_IS_PROD === 'true';
  }

  private static getClientId() {
    return JunoSubscription.isProd() ? process.env.SAAS_JUNO_PROD_CLIENT_ID : process.env.SAAS_JUNO_SANDBOX_CLIENT_ID;
  }

  private static getClientSecret() {
    return JunoSubscription.isProd()
      ? process.env.SAAS_JUNO_PROD_CLIENT_SECRET
      : process.env.SAAS_JUNO_SANDBOX_CLIENT_SECRET;
  }

  private static getJunoPublicKey() {
    return JunoSubscription.isProd() ? process.env.SAAS_JUNO_PROD_PUBLIC_KEY : process.env.SAAS_JUNO_SANDBOX_PUBLIC_KEY;
  }

  private static getJunoPrivateKey() {
    return JunoSubscription.isProd()
      ? process.env.SAAS_JUNO_PROD_PRIVATE_KEY
      : process.env.SAAS_JUNO_SANDBOX_PRIVATE_KEY;
  }

  private static getResourceUrl() {
    return JunoSubscription.isProd() ? process.env.SAAS_JUNO_PROD_RESOURCE : process.env.SAAS_JUNO_SANDBOX_RESOURCE;
  }

  private static getAuthUrl() {
    return JunoSubscription.isProd() ? process.env.SAAS_JUNO_PROD_AUTH : process.env.SAAS_JUNO_SANDBOX_AUTH;
  }

  private async getCardHash(
    ccNumber: string,
    name: string,
    ccCode: string,
    ccExpiresOnMonth: string,
    ccExpiresOnYear: string
  ): Promise<string> {
    return new Promise(async (resolve, reject) => {
      let tokenJuno = JunoSubscription.getJunoPublicKey();

      let cardData = {
        cardNumber: ccNumber,
        holderName: name,
        securityCode: ccCode,
        expirationMonth: ccExpiresOnMonth,
        expirationYear: ccExpiresOnYear
      };

      let checkout = new Juno.DirectCheckout(tokenJuno, JunoSubscription.isProd());

      checkout.getCardHash(cardData, resolve, reject);
    });
  }
}
