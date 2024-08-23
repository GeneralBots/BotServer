REM SET SCHEDULE "1 * * * * *"

REM Obtém token do Partner Center via token do AD. 

SET HEADER "return-client-request-id" AS "true"
SET HEADER "Content-Type" As "application/x-www-form-urlencoded; charset=utf-8"

pcToken  = POST "https://login.microsoftonline.com/" + tenantId +  "/oauth2/token", "resource=https%3A%2F%2Fgraph.windows.net&client_id=" + clientId  +  "&client_secret=" + clientSecret + "&grant_type=client_credentials"

REM repara chamada de Billing.

SET HEADER "Authorization" AS "Bearer " + pcToken.access_token
SET HEADER "MS-Contract-Version" AS "v1"
SET HEADER "MS-CorrelationId" AS uuid()
SET HEADER "MS-RequestId" AS uuid()
SET HEADER "MS-PartnerCenter-Application" AS "General Bots"
SET HEADER "X-Locale" AS "en-US"

REM Sincroniza Customers e Subscriptions
 
SET PAGE MODE "none"
list = GET host + "/v1/customers?size=20000"
MERGE "Customers" WITH list.items BY "Id"

FOR EACH item IN list
    subs = GET host + "/v1/customers/" + item.id + "/subscriptions"
    MERGE "Subscriptions" WITH  subs.items BY "Id"
END FOR


REM Determina período.

IF today = dueDay THEN
    IF period = "previous" AND NOT CONTINUATION TOKEN THEN
        period = "current"
    ELSE
        period = "previous"
    END IF
    ELSE
        period = "current"
END IF



REM Perform the call and loop through the billing items.

SET PAGE MODE "auto"
list = GET host + "/v1/invoices/unbilled/lineitems?provider=onetime&invoicelineitemtype=usagelineitems&currencycode=" +  currency + "&period=previous&idparceiro=" + idparceiro
FOR EACH item IN list
    SAVE "Billing", item.alternateId, item.availabilityId, item.billableQuantity, item.billingFrequency, item.chargeEndDate, item.chargeStartDate, item.chargeType, item.currency, item.customerCountry, item.customerDomainName, item.customerId, item.customerName, item.effectiveUnitPrice, item.invoiceNumber, item.meterDescription, item.mpnId, item.orderDate, item.orderId, item.partnerId, item.pCToBCExchangeRate, item.pCToBCExchangeRateDate, item.priceAdjustmentDescription, item.pricingCurrency, item.productId, item.productName, item.publisherId, item.publisherName, item.quantity, item.resellerMpnId, item.reservationOrderId, item.skuId, item.skuName, item.subscriptionDescription, item.subscriptionId, item.subtotal, item.taxTotal, item.termAndBillingCycle, item.totalForCustomer, item.unitPrice, item.unitType
END FOR


TABLE Billing
    CustomerId Customers
    ResourceGroup string(200)  
    ResourceUri string(1000) 
    Tags string(max) 
    AdditionalInfo string(max)
    ServiceInfo1 string(max) 
    ServiceInfo2 string(max) 
    CustomerCountry string(6)  
    MpnId string(50) 
    ResellerMpnId string(50)
    ChargeType string(200)
    UnitPrice* double
    Quantity* double
    UnitType string(max)
    BillingPreTaxTotal double
    BillingCurrency string(6)
    PricingPreTaxTotal double
    PricingCurrency string(6)
    EntitlementId string(50)
    EntitlementDescription string(400)
    PCToBCExchangeRate double
    PCToBCExchangeRateDate date
    EffectiveUnitPrice* double
    RateOfPartnerEarnedCredit double
    ConsumedService string(200)
    ResourceLocation string(100)
    MeterRegion string(100)
    PartnerId string(50)
    PartnerName string(400)
    CustomerName string(400)
    CustomerDomainName string(400)
    InvoiceNumber string(400)
    ProductId string(50)
    SkuId string(50)
    AvailabilityId string(50)
    SkuName string(200)
    ProductName string(400)
    PublisherName string(200)
    PublisherId string(200)
    SubscriptionId string(50)
    SubscriptionDescription string(400)
    ChargeStartDate* date
    ChargeEndDate* date
    UsageDate date
    MeterType string(400)
    MeterCategory string(100)
    MeterId string(50)
    MeterSubCategory string(100)
    MeterName string(200)
    UnitOfMeasure string(100)
    Reprocess boolean
END TABLE


TABLE Customers
    TenantId guid
    CompanyName string(100)
    Id guid
END TABLE


TABLE Subscriptions
    CustomerId Customers
    Id guid
    OfferName string(50)
END TABLE

