export BOT_ID=
./mc alias set minio http://localhost:9000 user pass
./mc admin user add minio $BOT_ID 
 
cat > $BOT_ID-policy.json <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:ListBucket",
        "s3:GetObject",
        "s3:PutObject",
        "s3:DeleteObject"
      ],
      "Resource": [
        "arn:aws:s3:::pragmatismo-$BOT_ID.gbai",
        "arn:aws:s3:::pragmatismo-$BOT_ID.gbai/*"
      ]
    }
  ]
}
EOF

./mc admin policy create minio $BOT_ID-policy $BOT_ID-policy.json
./mc admin policy attach minio $BOT_ID-policy --user $BOT_ID
