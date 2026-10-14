
#[test]
pub(super) fn bot_slug_normalizes() {
    assert_eq!(bot_slug("My Bot_Name"), "my-bot-name");
    assert_eq!(bot_slug("cristo"), "cristo");
    assert_eq!(bot_slug("weird!!name"), "weirdname");
}

#[test]
pub(super) fn alm_org_slug_is_url_safe() {
    assert_eq!(super::super::alm_org_from_slug("My Branch"), "mybranch");
    assert_eq!(super::super::alm_org_from_slug("cristo"), "cristo");
}
