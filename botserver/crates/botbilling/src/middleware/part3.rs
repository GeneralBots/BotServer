use super::*;

pub async fn organization_context_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let org_id = extract_organization_id(&request);

    if let Some(org_id) = org_id {
        let mut request = request;
        request.extensions_mut().insert(OrganizationContext {
            organization_id: org_id,
            user_id: None,
            plan_id: None,
        });
        next.run(request).await
    } else {
        next.run(request).await
    }
}
