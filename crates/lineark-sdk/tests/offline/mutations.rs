use super::*;

#[tokio::test]
async fn document_create_sends_input_variable() {
    use lineark_sdk::generated::inputs::DocumentCreateInput;

    let (server, client) = setup_mutation("documentCreate").await;
    let input = DocumentCreateInput {
        title: Some("Test Document".to_string()),
        content: Some("# Hello".to_string()),
        ..Default::default()
    };
    let _ = client.document_create::<Document>(input).await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["input"]["title"], "Test Document");
    assert_eq!(vars["input"]["content"], "# Hello");
}

#[tokio::test]
async fn document_update_sends_input_and_id() {
    use lineark_sdk::generated::inputs::DocumentUpdateInput;

    let (server, client) = setup_mutation("documentUpdate").await;
    let input = DocumentUpdateInput {
        title: Some("Updated Title".to_string()),
        ..Default::default()
    };
    let _ = client
        .document_update::<Document>(input, "doc-uuid-123".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["input"]["title"], "Updated Title");
    assert_eq!(vars["id"], "doc-uuid-123");
}

#[tokio::test]
async fn document_delete_sends_id() {
    let (server, client) = setup_mutation("documentDelete").await;
    let _ = client
        .document_delete::<Document>("doc-uuid-456".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "doc-uuid-456");
}

#[tokio::test]
async fn issue_relation_create_sends_input() {
    use lineark_sdk::generated::enums::IssueRelationType;
    use lineark_sdk::generated::inputs::IssueRelationCreateInput;

    let (server, client) = setup_mutation("issueRelationCreate").await;
    let input = IssueRelationCreateInput {
        issue_id: Some("issue-a".to_string()),
        related_issue_id: Some("issue-b".to_string()),
        r#type: Some(IssueRelationType::Blocks),
        ..Default::default()
    };
    let _ = client
        .issue_relation_create::<IssueRelation>(None, input)
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["input"]["issueId"], "issue-a");
    assert_eq!(vars["input"]["relatedIssueId"], "issue-b");
    assert_eq!(vars["input"]["type"], "blocks");
    assert_eq!(vars["overrideCreatedAt"], Value::Null);
}

#[tokio::test]
async fn file_upload_sends_required_params() {
    let (server, client) = setup_mutation("fileUpload").await;
    let _ = client
        .file_upload(
            None,
            Some(true),
            1024,
            "image/png".to_string(),
            "screenshot.png".to_string(),
        )
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["size"], 1024);
    assert_eq!(vars["contentType"], "image/png");
    assert_eq!(vars["filename"], "screenshot.png");
    assert_eq!(vars["makePublic"], true);
    assert_eq!(vars["metaData"], Value::Null);
}

#[tokio::test]
async fn image_upload_from_url_sends_url() {
    let (server, client) = setup_mutation("imageUploadFromUrl").await;
    let _ = client
        .image_upload_from_url("https://example.com/image.png".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["url"], "https://example.com/image.png");
}

#[tokio::test]
async fn issue_archive_sends_id_and_trash() {
    let (server, client) = setup_mutation("issueArchive").await;
    let _ = client
        .issue_archive::<Issue>(Some(true), "issue-uuid-arch".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-arch");
    assert_eq!(vars["trash"], true);
}

#[tokio::test]
async fn issue_archive_without_trash_sends_null() {
    let (server, client) = setup_mutation("issueArchive").await;
    let _ = client
        .issue_archive::<Issue>(None, "issue-uuid-arch2".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-arch2");
    assert_eq!(vars["trash"], Value::Null);
}

#[tokio::test]
async fn issue_unarchive_sends_id() {
    let (server, client) = setup_mutation("issueUnarchive").await;
    let _ = client
        .issue_unarchive::<Issue>("issue-uuid-unarch".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-unarch");
}

#[tokio::test]
async fn issue_delete_sends_id_and_permanently_delete() {
    let (server, client) = setup_mutation("issueDelete").await;
    let _ = client
        .issue_delete::<Issue>(Some(true), "issue-uuid-123".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-123");
    assert_eq!(vars["permanentlyDelete"], true);
}

#[tokio::test]
async fn issue_delete_without_permanently_sends_null() {
    let (server, client) = setup_mutation("issueDelete").await;
    let _ = client
        .issue_delete::<Issue>(None, "issue-uuid-456".to_string())
        .await;
    let vars = extract_variables(&server.received_requests().await.unwrap());
    assert_eq!(vars["id"], "issue-uuid-456");
    assert_eq!(vars["permanentlyDelete"], Value::Null);
}
