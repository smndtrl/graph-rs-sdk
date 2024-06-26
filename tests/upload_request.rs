use bytes::BytesMut;
use graph_rs_sdk::http::{BodyRead, FileConfig};
use graph_rs_sdk::*;
use std::time::Duration;
use test_tools::oauth_request::DEFAULT_CLIENT_CREDENTIALS_MUTEX3;

async fn get_special_folder_id(user_id: &str, folder: &str, client: &Graph) -> GraphResult<String> {
    let response = client
        .user(user_id)
        .drive()
        .get_special(folder)
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    let parent_reference_id = body["id"].as_str().unwrap();
    Ok(parent_reference_id.into())
}

async fn upload_new_file(
    user_id: &str,
    parent_reference_id: &str,
    file_name: &str,
    local_file: &str,
    client: &Graph,
) -> GraphResult<reqwest::Response> {
    let file_config = FileConfig::new(local_file);
    let bytes_mut: BytesMut = file_config.try_into()?;
    let reader = BodyRead::try_from(bytes_mut)?;

    client
        .drive(user_id)
        .item(parent_reference_id)
        .upload_items_content(file_name, reader)
        .send()
        .await
}

async fn upload_file_reqwest_body(
    user_id: &str,
    parent_reference_id: &str,
    file_name: &str,
    local_file: &str,
    client: &Graph,
) -> GraphResult<reqwest::Response> {
    let file = tokio::fs::File::open(local_file).await?;
    let body = reqwest::Body::from(file);

    client
        .drive(user_id)
        .item(parent_reference_id)
        .upload_items_content(file_name, body)
        .send()
        .await
}

async fn delete_file(
    user_id: &str,
    item_id: &str,
    client: &Graph,
) -> GraphResult<reqwest::Response> {
    client
        .user(user_id)
        .drive()
        .item(item_id)
        .delete_items()
        .send()
        .await
}

async fn get_file_content(
    user_id: &str,
    item_id: &str,
    client: &Graph,
) -> GraphResult<reqwest::Response> {
    client
        .user(user_id)
        .drive()
        .item(item_id)
        .get_items_content()
        .send()
        .await
}

#[tokio::test]
async fn upload_bytes_mut() {
    let test_client = DEFAULT_CLIENT_CREDENTIALS_MUTEX3.lock().await;
    let local_file = "./test_files/test_upload_file_bytes.txt";
    let file_name = ":/test_upload_file_bytes.txt:";

    let parent_reference_id = get_special_folder_id(
        test_client.user_id.as_str(),
        "Documents",
        &test_client.client,
    )
    .await
    .unwrap();
    let upload_res = upload_new_file(
        test_client.user_id.as_str(),
        parent_reference_id.as_str(),
        file_name,
        local_file,
        &test_client.client,
    )
    .await;

    if let Ok(response) = upload_res {
        assert!(response.status().is_success());
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["id"].as_str().is_some());
        let item_id = body["id"].as_str().unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        let response = get_file_content(test_client.user_id.as_str(), item_id, &test_client.client)
            .await
            .unwrap();
        assert!(response.status().is_success());

        let text = response.text().await.unwrap();
        assert_eq!("Upload Bytes", text.trim());

        let delete_res =
            delete_file(test_client.user_id.as_str(), item_id, &test_client.client).await;

        if let Ok(response) = delete_res {
            assert!(response.status().is_success());
        } else if let Err(err) = delete_res {
            panic!("Request Error. Method: drive delete. Error: {err:#?}");
        }
    } else if let Err(err) = upload_res {
        panic!("Request Error. Method: drive upload. Error: {err:#?}");
    }
}

#[tokio::test]
async fn upload_reqwest_body() {
    let test_client = DEFAULT_CLIENT_CREDENTIALS_MUTEX3.lock().await;
    let local_file = "./test_files/test_upload_file_bytes.txt";
    let file_name = ":/test_upload_file_bytes.txt:";

    let parent_reference_id = get_special_folder_id(
        test_client.user_id.as_str(),
        "Documents",
        &test_client.client,
    )
    .await
    .unwrap();
    let upload_res = upload_file_reqwest_body(
        test_client.user_id.as_str(),
        parent_reference_id.as_str(),
        file_name,
        local_file,
        &test_client.client,
    )
    .await;

    if let Ok(response) = upload_res {
        assert!(response.status().is_success());
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["id"].as_str().is_some());
        let item_id = body["id"].as_str().unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        let response = get_file_content(test_client.user_id.as_str(), item_id, &test_client.client)
            .await
            .unwrap();
        assert!(response.status().is_success());

        let text = response.text().await.unwrap();
        assert_eq!("Upload Bytes", text.trim());

        let delete_res =
            delete_file(test_client.user_id.as_str(), item_id, &test_client.client).await;

        if let Ok(response) = delete_res {
            assert!(response.status().is_success());
        } else if let Err(err) = delete_res {
            panic!("Request Error. Method: drive delete. Error: {err:#?}");
        }
    } else if let Err(err) = upload_res {
        panic!("Request Error. Method: drive upload. Error: {err:#?}");
    }
}
