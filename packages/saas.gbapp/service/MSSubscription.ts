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

"use strict"

import { MainService } from "./MainService.js";

export class MSSubscriptionService {

    public async handleMSHook(req: any, res: any) {

    }

    public async handleMSSignUp(req: any, res: any) {
        let token = req.params.token;
        let url = `https://marketplaceapi.microsoft.com/api/saas/subscriptions/resolve?api-version=2018-08-31`;
        let options = {
            uri: url,
            method: 'GET',
            headers: {
                'x-ms-marketplace-token': token
            }
        };
        let result = null; // TODO: await fetch({});
        let data = JSON.parse(result);

        const additionalData = {
            "id": "<guid>",  // purchased SaaS subscription ID 
            "subscriptionName": "Contoso Cloud Solution", // SaaS subscription name 
            "offerId": "offer1", // purchased offer ID
            "planId": "silver", // purchased offer's plan ID
            "quantity": "20", // number of purchased seats, might be empty if the plan is not per seat
            "subscription": { // full SaaS subscription details, see Get Subscription APIs response body for full description
                "id": "<guid>",
                "publisherId": "contoso",
                "offerId": "offer1",
                "name": "Contoso Cloud Solution",
                "saasSubscriptionStatus": " PendingFulfillmentStart ",
                "beneficiary": {
                    "emailId": "test@test.com",
                    "objectId": "<guid>",
                    "tenantId": "<guid>",
                    "pid": "<ID of the user>"
                },
                "purchaser": {
                    "emailId": "test@test.com",
                    "objectId": "<guid>",
                    "tenantId": "<guid>",
                    "pid": "<ID of the user>"
                },
                "planId": "silver",
                "term": {
                    "termUnit": "P1M",
                    "startDate": "2019 - 05 - 31",
                    "endDate": "2019-06-29",
                },
                "isTest": true,
                "isFreeTrial": false,
                "allowedCustomerOperations": [
                    "Delete",
                    "Update",
                    "Read"
                ],
                "sandboxType": "None",
                "sessionMode": "None"
            }
        }

        const service = new MainService();
        service.createSubscriptionMSFT("email", "plan", "offer",
            Number.parseInt(additionalData.quantity), additionalData);

        url = `https://marketplaceapi.microsoft.com/api/saas/subscriptions/${data.id}?api-version=2018-08-31`;
        options = {
            uri: url,
            method: 'GET',
            headers: {
                'x-ms-marketplace-token': token
            }
        };

         result = null; //  TODO: fetch.
         data = JSON.parse(result);
    }

    public async handleMSLanding(req: any, res: any) {

    }

    public async Unsubscribe() {

    }
    public async Suspend() {

    }
    public async Reinstateou() {

    }
    public async ChangePlan() {

    }
    public async ChangeQuantity() {

    }
    public async Transfer() {

    }
}