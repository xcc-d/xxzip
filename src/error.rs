use std::fmt;
use std::io;
use std::path::StripPrefixError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum ZipError {
    Io(io::Error),
    Zip(zip::result::ZipError),
    StripPrefix(StripPrefixError),
    InvalidPath(String),
    Utf8Error(FromUtf8Error),
    WalkDir(walkdir::Error),
    Other(String),
}

impl fmt::Display for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZipError::Io(err) => write!(f, "IO error: {}", err),
            ZipError::Zip(err) => write!(f, "Zip error: {}", err),
            ZipError::StripPrefix(err) => write!(f, "Path prefix error: {}", err),
            ZipError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            ZipError::Utf8Error(err) => write!(f, "UTF-8 conversion error: {}", err),
            ZipError::WalkDir(err) => write!(f, "Directory traversal error: {}", err),
            ZipError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ZipError {}

impl From<io::Error> for ZipError {
    fn from(err: io::Error) -> Self {
        ZipError::Io(err)
    }
}

impl From<zip::result::ZipError> for ZipError {
    fn from(err: zip::result::ZipError) -> Self {
        ZipError::Zip(err)
    }
}

impl From<StripPrefixError> for ZipError {
    fn from(err: StripPrefixError) -> Self {
        ZipError::StripPrefix(err)
    }
}

impl From<FromUtf8Error> for ZipError {
    fn from(err: FromUtf8Error) -> Self {
        ZipError::Utf8Error(err)
    }
}

impl From<walkdir::Error> for ZipError {
    fn from(err: walkdir::Error) -> Self {
        ZipError::WalkDir(err)
    }
}

impl From<String> for ZipError {
    fn from(err: String) -> Self {
        ZipError::Other(err)
    }
}

impl From<&str> for ZipError {
    fn from(err: &str) -> Self {
        ZipError::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn test_error_display() {
        // 测试IO错误转换
        let io_err = io::Error::new(io::ErrorKind::NotFound, "文件未找到");
        let zip_err: ZipError = io_err.into();
        assert!(format!("{}", zip_err).contains("IO error"));
        
        // 测试路径错误转换
        let path_err = ZipError::InvalidPath("invalid/path".to_string());
        assert!(format!("{}", path_err).contains("Invalid path"));
        
        // 测试字符串错误转换
        let str_err: ZipError = "测试错误".into();
        assert!(format!("{}", str_err).contains("测试错误"));
    }

    #[test]
    fn test_from_impls() {
        // 测试从&str转换
        let err: ZipError = "错误信息".into();
        if let ZipError::Other(msg) = err {
            assert_eq!(msg, "错误信息");
        } else {
            panic!("转换失败");
        }
        
        // 测试从String转换
        let err: ZipError = "错误信息".to_string().into();
        if let ZipError::Other(msg) = err {
            assert_eq!(msg, "错误信息");
        } else {
            panic!("转换失败");
        }
        
        // 测试从StripPrefixError转换
        let base = PathBuf::from("/base");
        let path = PathBuf::from("/other");
        let strip_err = path.strip_prefix(&base).unwrap_err();
        let err: ZipError = strip_err.into();
        if let ZipError::StripPrefix(_) = err {
            // 成功
        } else {
            panic!("转换失败");
        }
    }
} 