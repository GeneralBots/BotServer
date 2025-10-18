REM ============================================================================
REM Pricing Tool with Knowledge Base and Website Integration
REM ============================================================================
REM This example demonstrates:
REM 1. Product pricing lookup from CSV database
REM 2. Integration with product brochures KB
REM 3. Dynamic website content indexing
REM 4. Multi-source knowledge retrieval
REM ============================================================================

REM Define tool parameters
PARAM product AS string LIKE "fax" DESCRIPTION "Required name of the product you want to inquire about."

REM Tool description
DESCRIPTION "Whenever someone asks for a price, call this tool and return the price of the specified product name. Also provides access to product documentation and specifications."

REM ============================================================================
REM Validate Input
REM ============================================================================

IF product = "" THEN
    TALK "Please specify which product you would like to know the price for."
    EXIT
END IF

REM Normalize product name (lowercase for case-insensitive search)
product_normalized = LOWER(TRIM(product))

PRINT "Looking up pricing for product: " + product_normalized

REM ============================================================================
REM Search Product Database
REM ============================================================================

price = -1
stock_status = "unknown"
product_category = ""
product_description = ""

REM Search in products CSV file
productRecord = FIND "products.csv", "LOWER(name) = '" + product_normalized + "'"

IF productRecord THEN
    price = productRecord.price
    stock_status = productRecord.stock_status
    product_category = productRecord.category
    product_description = productRecord.description

    PRINT "Product found in database:"
    PRINT "  Name: " + productRecord.name
    PRINT "  Price: $" + STR(price)
    PRINT "  Stock: " + stock_status
    PRINT "  Category: " + product_category
ELSE
    REM Product not found in database
    PRINT "Product not found in local database: " + product

    TALK "I couldn't find the product '" + product + "' in our catalog. Please check the spelling or ask about a different product."

    REM Still activate KB in case user wants to browse catalog
    ADD_KB "productbrochurespdfsanddocs"

    RETURN -1
END IF

REM ============================================================================
REM Add Product Documentation Knowledge Base
REM ============================================================================
REM The .gbkb/productbrochurespdfsanddocs folder should contain:
REM - product_catalog.pdf
REM - technical_specifications.pdf
REM - user_manuals.pdf
REM - warranty_information.pdf
REM - comparison_charts.pdf
REM ============================================================================

ADD_KB "productbrochurespdfsanddocs"

REM ============================================================================
REM Add Product Website for Real-time Information
REM ============================================================================
REM This indexes the product's official page with:
REM - Latest specifications
REM - Customer reviews
REM - Installation guides
REM - Troubleshooting tips
REM ============================================================================

product_url = "https://example.com/products/" + product_normalized

REM Try to add website (will only work if URL is accessible)
REM ADD_WEBSITE product_url

REM Alternative: Add general product documentation page
ADD_WEBSITE "https://example.com/docs/products"

PRINT "Knowledge base activated for: " + product

REM ============================================================================
REM Build Response Message
REM ============================================================================

response_message = "**Product Information: " + productRecord.name + "**\n\n"
response_message = response_message + "💰 **Price:** $" + STR(price) + "\n"
response_message = response_message + "📦 **Availability:** " + stock_status + "\n"
response_message = response_message + "📂 **Category:** " + product_category + "\n\n"

IF product_description <> "" THEN
    response_message = response_message + "📝 **Description:**\n" + product_description + "\n\n"
END IF

REM Add stock availability message
IF stock_status = "in_stock" THEN
    response_message = response_message + "✅ This product is currently in stock and ready to ship!\n\n"
ELSE IF stock_status = "low_stock" THEN
    response_message = response_message + "⚠️ Limited availability - only a few units left in stock.\n\n"
ELSE IF stock_status = "out_of_stock" THEN
    response_message = response_message + "❌ Currently out of stock. Expected restock date: contact sales.\n\n"
ELSE IF stock_status = "pre_order" THEN
    response_message = response_message + "🔜 Available for pre-order. Ships when available.\n\n"
END IF

REM Inform about available knowledge
response_message = response_message + "📚 **Need More Information?**\n"
response_message = response_message + "I now have access to our complete product documentation. You can ask me:\n\n"
response_message = response_message + "• What are the technical specifications?\n"
response_message = response_message + "• How does it compare to other products?\n"
response_message = response_message + "• What's included in the warranty?\n"
response_message = response_message + "• Are there any setup instructions?\n"
response_message = response_message + "• What do customers say about this product?\n"

TALK response_message

REM ============================================================================
REM Store Product Context in Bot Memory
REM ============================================================================

SET BOT MEMORY "last_product_inquiry", product_normalized
SET BOT MEMORY "last_product_price", STR(price)
SET BOT MEMORY "last_product_category", product_category
SET BOT MEMORY "inquiry_timestamp", NOW()

REM ============================================================================
REM Set User Context for Personalized Follow-up
REM ============================================================================

SET CONTEXT "current_product", product_normalized
SET CONTEXT "current_price", STR(price)
SET CONTEXT "browsing_category", product_category

REM ============================================================================
REM Log Inquiry for Analytics
REM ============================================================================

inquiry_id = UUID()
inquiry_date = NOW()
user_session = SESSION_ID()

SAVE "product_inquiries.csv", inquiry_id, user_session, product_normalized, price, inquiry_date

PRINT "Inquiry logged: " + inquiry_id

REM ============================================================================
REM Check for Related Products
REM ============================================================================

IF product_category <> "" THEN
    PRINT "Searching for related products in category: " + product_category

    related_products = FIND ALL "products.csv", "category = '" + product_category + "' AND LOWER(name) <> '" + product_normalized + "'"

    IF related_products <> NULL AND LEN(related_products) > 0 THEN
        related_message = "\n\n**Related Products You Might Like:**\n\n"

        counter = 0
        FOR EACH related IN related_products
            IF counter < 3 THEN
                related_message = related_message + "• " + related.name + " - $" + STR(related.price)

                IF related.stock_status = "in_stock" THEN
                    related_message = related_message + " ✅"
                END IF

                related_message = related_message + "\n"
                counter = counter + 1
            END IF
        NEXT

        TALK related_message
    END IF
END IF

REM ============================================================================
REM Optional: Check for Promotions
REM ============================================================================

promotion = FIND "promotions.csv", "LOWER(product_name) = '" + product_normalized + "' AND active = true"

IF promotion THEN
    promo_message = "\n\n🎉 **Special Offer!**\n"
    promo_message = promo_message + promotion.description + "\n"
    promo_message = promo_message + "Discount: " + promotion.discount_percentage + "%\n"
    promo_message = promo_message + "Valid until: " + promotion.end_date + "\n"

    discounted_price = price * (1 - (promotion.discount_percentage / 100))
    promo_message = promo_message + "\n**Discounted Price: $" + STR(discounted_price) + "**"

    TALK promo_message

    SET BOT MEMORY "active_promotion", promotion.code
END IF

REM ============================================================================
REM Return the price for programmatic use
REM ============================================================================

RETURN price
