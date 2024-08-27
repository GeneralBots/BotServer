REM Set SCHEDULE "1 * * * * *"

REM Obtém token Do Partner Center via token Do AD.

Set HEADER "return-client-request-id" As "True"
Set HEADER "Content-Type" As "application/x-www-form-urlencoded; charset=utf-8"
REM pcToken = POST "https://login.microsoftonline.com/" + tenantId + "/oauth2/token", "resource=https%3A%2F%2Fgraph.windows.net&client_id=" + clientId + "&client_secret=" + clientSecret + "&grant_type=client_credentials"

REM repara chamada de Billing.

REM Set HEADER "Authorization" As "Bearer " + pcToken.access_token
Set HEADER "MS-Contract-Version" As "v1"
Set HEADER "MS-CorrelationId" As uuid()
Set HEADER "MS-RequestId" As uuid()
Set HEADER "MS-PartnerCenter-Application" As "VPN General Bots"
Set HEADER "X-Locale" As "en-US"

REM Syncs Customers and Subscriptions.

Set PAGE MODE "none"
list = Get host + "/v1/customers?size=20000"

MERGE "Customers" With list.items BY "Id"

FOR EACH item IN list
    
subs = Get host + "/v1/customers/" + item.id + "/subscriptions"
    MERGE "Subscriptions" With subs.items BY "Id"
    END For
    
    REM Check period.

    If today = dueDay Then
        If period = "previous" And Not CONTINUATION TOKEN Then
            period = "current"
        Else
            period = "previous"
        End If
    Else
        period = "current"
    End If
    
    REM Perform the Call And Loop through the billing items.
    
    Set PAGE MODE "auto"
    list = Get host + "/v1/invoices/unbilled/lineitems?provider=onetime&invoicelineitemtype=usagelineitems&currencycode=" + currency + "&period=previous&idparceiro=" + idparceiro
    For EACH item IN list
        SAVE "Billing", item.alternateId, item.availabilityId, item.billableQuantity, item.billingFrequency, item.chargeEndDate, item.chargeStartDate, item.chargeType, item.currency, item.customerCountry, item.customerDomainName, item.customerId, item.customerName, item.effectiveUnitPrice, item.invoiceNumber, item.meterDescription, item.mpnId, item.orderDate, item.orderId, item.partnerId, item.pCToBCExchangeRate, item.pCToBCExchangeRateDate, item.priceAdjustmentDescription, item.pricingCurrency, item.productId, item.productName, item.publisherId, item.publisherName, item.quantity, item.resellerMpnId, item.reservationOrderId, item.skuId, item.skuName, item.subscriptionDescription, item.subscriptionId, item.subtotal, item.taxTotal, item.termAndBillingCycle, item.totalForCustomer, item.unitPrice, item.unitType
    END For

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